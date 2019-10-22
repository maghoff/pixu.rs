use chrono::{Duration, Utc};
use diesel;
use diesel::sqlite::SqliteConnection;
use futures::task::{Spawn, SpawnExt};
use futures::{compat::Stream01CompatExt, FutureExt, TryStreamExt};
use jsonwebtoken::Header;
use lettre::{SmtpTransport, Transport};
use lettre_email::{EmailBuilder, Mailbox};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use serde_derive::{Deserialize, Serialize};
use serde_urlencoded;
use std::sync::{Arc, Mutex};
use web::{Cookie, FutureBox, MediaType, RepresentationBox, Resource, Response};

use super::super::handling_error::HandlingError;
use super::{AuthPhase, ValidationClaims};

#[derive(Serialize)]
struct ValidationArgs<'a> {
    claims: &'a str,
    redirect: &'a str,
}

#[derive(Deserialize)]
struct PostArgs {
    email: String,
    redirect: String,
}

#[derive(BartDisplay)]
#[template = "templates/auth-step1.html"]
struct Post<'a> {
    email: &'a str,
}

pub struct InitiateAuth<S: Spawn + Send + 'static> {
    pub key: Vec<u8>,
    pub base_url: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub mailer: Arc<Mutex<SmtpTransport>>,
    pub sender: Mailbox,
    pub spawn: S,
}

fn is_registered_user(_email: &str) -> bool {
    // TODO implement stub

    true
}

async fn maybe_send_email<'a>(
    base_url: &'a str,
    email: String,
    claims: &'a str,
    mailer: Arc<Mutex<SmtpTransport>>,
    sender: Mailbox,
    redirect: &'a str,
) {
    if !is_registered_user(&email) {
        return;
    }

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

impl<S: Spawn + Send + 'static> InitiateAuth<S> {
    async fn issue(
        key: &[u8],
        base_url: String,
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
        let token = jsonwebtoken::encode(&Header::default(), &claims, key).unwrap();

        let mut parts = token.split('.');

        let head = parts.next().unwrap();
        let claims = parts.next().unwrap().to_string();
        let sign = parts.next().unwrap();

        spawn
            .spawn(async {
                let base_url = base_url;
                let claims = claims;
                let redirect = redirect;
                maybe_send_email(&base_url, email, &claims, mailer, sender, &redirect).await;
            })
            .unwrap();

        format!("{}.{}", head, sign)
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
        let cookie = Self::issue(
            &self.key,
            self.base_url,
            &email,
            self.mailer,
            self.sender,
            self.spawn,
            args.redirect,
        )
        .await;
        let cookie = Cookie::build("let-me-in", cookie).http_only(true).finish();

        Ok(Response {
            status: web::Status::Ok,
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