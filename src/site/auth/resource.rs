use chrono::{DateTime, Duration, Utc};
use diesel;
use diesel::sqlite::SqliteConnection;
use futures::task::{Spawn, SpawnExt};
use futures::{compat::Stream01CompatExt, FutureExt, TryStreamExt};
use hyper::http;
use jsonwebtoken::{encode, Header};
use lettre::{SmtpTransport, Transport};
use lettre_email::{EmailBuilder, Mailbox};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use serde_derive::{Deserialize, Serialize};
use serde_urlencoded;
use std::sync::{Arc, Mutex};
use web::{Cookie, CookieHandler, FutureBox, MediaType, RepresentationBox, Resource, Response};

use super::super::handling_error::HandlingError;

const KEY: &[u8] = b"secret";

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
struct NumberDate(i64);

impl From<DateTime<Utc>> for NumberDate {
    fn from(datetime: DateTime<Utc>) -> Self {
        NumberDate(datetime.timestamp())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum AuthPhase {
    Validation,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidationClaims {
    phase: AuthPhase,
    sub: String,
    exp: NumberDate,
    jti: u32,
}

pub struct InitiateAuth<S: Spawn + Send + 'static> {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub mailer: Arc<Mutex<SmtpTransport>>,
    pub sender: Mailbox,
    pub spawn: S,
}

pub struct VerifyAuth {
    claims: String,
    head_sign: String,
    redirect: String,
}

#[derive(BartDisplay)]
#[template = "templates/auth-step0.html"]
struct Get<'a> {
    claims: &'a str,
    head_sign: &'a str,
    redirect: &'a str,
    x: String,
}

#[derive(serde_derive::Deserialize)]
struct PostArgs {
    email: String,
    redirect: String,
}

#[derive(BartDisplay)]
#[template = "templates/auth-step1.html"]
struct Post<'a> {
    email: &'a str,
}

fn is_registered_user(_email: &str) -> bool {
    // TODO implement stub

    true
}

#[derive(Serialize)]
struct ValidationArgs<'a> {
    claims: &'a str,
    redirect: &'a str,
}

#[derive(Deserialize)]
pub struct ValidationArgsOwned {
    claims: String,
    redirect: String,
}

async fn maybe_send_email<'a>(
    email: String,
    claims: &'a str,
    mailer: Arc<Mutex<SmtpTransport>>,
    sender: Mailbox,
    redirect: &'a str,
) {
    if !is_registered_user(&email) {
        return;
    }

    let base_url = "http://127.0.0.1:1212/"; // FIXME

    let args = serde_urlencoded::to_string(ValidationArgs { claims, redirect }).unwrap();
    let verification_link = format!("{}verify_auth?{}", base_url, args);

    let email = EmailBuilder::new()
        .to(email)
        .from(sender)
        .subject("Innlogging")
        .text(format!("Følg denne linken: {}", verification_link))
        .build()
        .unwrap();

    let mut mailer = mailer.lock().expect("Don't know what to do about Poison");
    mailer.send(email.into()).unwrap();
}

fn verify_core(
    head_sign: &str,
    claims: &str,
) -> Result<ValidationClaims, Box<dyn std::error::Error>> {
    use jsonwebtoken::{Algorithm, Validation};
    const KEY: &[u8] = b"secret";

    let mut head_sign = head_sign.splitn(2, '.');
    let head = head_sign.next().unwrap();
    let sign = head_sign.next().ok_or("Missing . in head_sign")?;

    let token = format!("{}.{}.{}", head, claims, sign);

    let token = jsonwebtoken::decode::<ValidationClaims>(
        &token,
        KEY,
        &Validation {
            algorithms: vec![Algorithm::HS256],
            ..Default::default()
        },
    )?;

    if token.claims.phase == AuthPhase::Validation {
        Ok(token.claims)
    } else {
        Err("Wrong AuthPhase".into())
    }
}

impl<S: Spawn + Send + 'static> InitiateAuth<S> {
    async fn issue(
        email: impl ToString,
        mailer: Arc<Mutex<SmtpTransport>>,
        sender: Mailbox,
        mut spawn: S,
        redirect: String,
    ) -> String {
        let email = email.to_string();

        let claims = ValidationClaims {
            phase: AuthPhase::Validation,
            sub: email.clone(),
            exp: (Utc::now() + Duration::hours(2)).into(),
            jti: rand::random(),
        };
        let token = jsonwebtoken::encode(&Header::default(), &claims, KEY).unwrap();

        let mut parts = token.split('.');

        let head = parts.next().unwrap();
        let claims = parts.next().unwrap().to_string();
        let sign = parts.next().unwrap();

        spawn
            .spawn(async {
                let claims = claims;
                let redirect = redirect;
                maybe_send_email(email, &claims, mailer, sender, &redirect).await;
            })
            .unwrap();

        format!("{}.{}", head, sign)
    }

    async fn _old_try_post(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> Result<Response, HandlingError> {
        let content_type = content_type;
        if content_type != "application/x-www-form-urlencoded" {
            return Err(HandlingError::BadRequest(
                "Unacceptable Content-Type, must be application/x-www-form-urlencoded",
            ));
        }

        let body = body
            .compat()
            .try_concat()
            .await
            .map_err(|_| HandlingError::InternalServerError)?;
        let args: PostArgs = serde_urlencoded::from_bytes(&body)
            .map_err(|_| HandlingError::BadRequest("Invalid data"))?; // TODO Use given error.to_string()

        #[derive(serde_derive::Serialize)]
        struct Claims<'a> {
            sub: &'a str,
        }
        let claims = Claims { sub: &args.email };

        let token = encode(&Header::default(), &claims, "secret".as_ref()).unwrap();
        let cookie = Cookie::build("let-me-in", token).http_only(true).finish();

        Ok(Response {
            status: http::StatusCode::OK,
            representations: vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(Post { email: &args.email }.to_string()) as RepresentationBox
                }) as _,
            )],
            cookies: vec![cookie],
        })
    }

    async fn try_post(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> Result<Response, HandlingError> {
        let content_type = content_type;
        if content_type != "application/x-www-form-urlencoded" {
            return Err(HandlingError::BadRequest(
                "Unacceptable Content-Type, must be application/x-www-form-urlencoded",
            ));
        }

        let body = body
            .compat()
            .try_concat()
            .await
            .map_err(|_| HandlingError::InternalServerError)?;
        let args: PostArgs = serde_urlencoded::from_bytes(&body)
            .map_err(|_| HandlingError::BadRequest("Invalid data"))?; // TODO Use given error.to_string()

        let email = args.email;
        let cookie = Self::issue(&email, self.mailer, self.sender, self.spawn, args.redirect).await;
        let cookie = Cookie::build("let-me-in", cookie).http_only(true).finish();

        Ok(Response {
            status: http::StatusCode::OK,
            representations: vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || Box::new(Post { email: &email }.to_string()) as RepresentationBox)
                    as _,
            )],
            cookies: vec![cookie],
        })
    }

    async fn post_core(self: Box<Self>, content_type: String, body: hyper::Body) -> Response {
        self.try_post(content_type, body)
            .await
            .unwrap_or_else(|e| e.render())
    }
}

impl<S: Spawn + Send + 'static> Resource for InitiateAuth<S> {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        unimplemented!()
    }

    fn post<'a>(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> FutureBox<'a, Response> {
        self.post_core(content_type, body).boxed()
    }
}

impl VerifyAuth {
    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        let claims = verify_core(&self.head_sign, &self.claims);
        let x = format!("{:?}", claims);

        // TODO: Depending on `claims`,
        //  - respond with error message, or
        //  - issue login cookie and respond with redirect

        Ok(Response::new(
            http::StatusCode::OK,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(
                        Get {
                            claims: &self.claims,
                            head_sign: &self.head_sign,
                            redirect: &self.redirect,
                            x,
                        }
                        .to_string(),
                    ) as RepresentationBox
                }) as _,
            )],
        ))
    }

    async fn get_core(self: Box<Self>) -> Response {
        self.try_get().await.unwrap_or_else(|e| e.render())
    }
}

impl Resource for VerifyAuth {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        self.get_core().boxed()
    }
}

struct VerifyAuthCookieHandler {
    claims: String,
    redirect: String,
}

impl VerifyAuthCookieHandler {
    async fn async_cookies<'a>(
        self: Box<Self>,
        values: &'a [Option<&'a str>],
    ) -> Result<Box<dyn web::Resource + Send + 'static>, web::Error> {
        let cookie = values[0].ok_or(web::Error::BadRequest)?.to_string();

        Ok(Box::new(VerifyAuth {
            claims: self.claims,
            redirect: self.redirect,
            head_sign: cookie,
        }) as _)
    }
}

impl CookieHandler for VerifyAuthCookieHandler {
    fn read_cookies(&self) -> &[&str] {
        &["let-me-in"]
    }

    fn cookies<'a>(
        self: Box<Self>,
        values: &'a [Option<&'a str>],
    ) -> FutureBox<'a, Result<Box<dyn web::Resource + Send + 'static>, web::Error>> {
        self.async_cookies(values).boxed() as _
    }
}

pub struct VerifyAuthArgsConsumer;

impl crate::site::query_args::QueryArgsConsumer for VerifyAuthArgsConsumer {
    type Args = ValidationArgsOwned;

    fn args(self, args: Self::Args) -> Result<Box<dyn web::CookieHandler + Send>, web::Error> {
        Ok(Box::new(VerifyAuthCookieHandler {
            claims: args.claims,
            redirect: args.redirect,
        }))
    }
}
