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
use web::{Cookie, Error, FutureBox, MediaType, RepresentationBox, Resource, Response};

use super::super::handling_error::HandlingError;
use super::{Claims, ClaimsConsumer};

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

pub struct Auth<S: Spawn + Send + 'static> {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    mailer: Arc<Mutex<SmtpTransport>>,
    sender: Mailbox,
    spawn: S,
    claims: Option<Claims>,
}

#[derive(BartDisplay)]
#[template = "templates/auth-step0.html"]
struct Get<'a> {
    claims: &'a Option<Claims>,
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
    let verification_link = format!("{}auth?{}", base_url, args);

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

impl<S: Spawn + Send + 'static> Auth<S> {
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

    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        // TODO Accept ValiationArgs as query args
        //  -> If present, perform validation
        //  -> Is there a valid "else" case?

        Ok(Response::new(
            http::StatusCode::OK,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(
                        Get {
                            claims: &self.claims,
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

impl<S: Spawn + Send + 'static> Resource for Auth<S> {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        self.get_core().boxed()
    }

    fn post<'a>(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> FutureBox<'a, Response> {
        self.post_core(content_type, body).boxed()
    }
}

pub struct AuthLoader<S: Spawn + Send + 'static> {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub mailer: Arc<Mutex<SmtpTransport>>,
    pub sender: Mailbox,
    pub spawn: S,
}

impl<S: Spawn + Send + 'static> ClaimsConsumer for AuthLoader<S> {
    type Claims = Claims;

    fn claims<'a>(
        self,
        claims: Option<Self::Claims>,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>> {
        async {
            Ok(Box::new(Auth {
                claims,
                db_pool: self.db_pool,
                mailer: self.mailer,
                sender: self.sender,
                spawn: self.spawn,
            }) as Box<dyn Resource + Send + 'static>)
        }
            .boxed() as _
    }
}
