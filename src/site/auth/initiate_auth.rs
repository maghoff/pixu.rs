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
#[template = "templates/initiate-auth.html"]
struct Post<'a> {
    email: &'a str,
}

pub struct InitiateAuth<S: Spawn + Send + 'static> {
    pub title: String,
    pub key: Vec<u8>,
    pub base_url: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub mailer: Arc<Mutex<SmtpTransport>>,
    pub sender: Mailbox,
    pub spawn: S,
}

fn is_registered_user_core(
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    email: &str,
) -> Result<bool, String> {
    use crate::db::schema::pixur_authorizations::dsl::*;
    use diesel::dsl::*;
    use diesel::prelude::*;

    let db_connection = db_pool
        .get()
        .map_err(|e| format!("Unable to get db connection: {}", e))?;

    let exists = select(exists(pixur_authorizations.filter(sub.eq(email))))
        .first::<bool>(&*db_connection)
        .map_err(|e| format!("Unable to get db result: {}", e))?;

    Ok(exists)
}

fn is_registered_user(db_pool: Pool<ConnectionManager<SqliteConnection>>, email: &str) -> bool {
    match is_registered_user_core(db_pool, email) {
        Ok(x) => {
            eprintln!("is_registered_user({:?}): {}", email, x);
            x
        }
        Err(e) => {
            eprintln!("is_registered_user({:?}): {}", email, e);
            false
        }
    }
}

async fn maybe_send_email<'a>(
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    base_url: &'a str,
    email: String,
    claims: &'a str,
    mailer: Arc<Mutex<SmtpTransport>>,
    sender: Mailbox,
    redirect: &'a str,
) {
    // Giving an unknown user a valid login cookie is not a problem, since
    // authorization is done per resource after login. However, there is
    // no advantage to logging in unknown users, so let's not.
    if !is_registered_user(db_pool, &email) {
        return;
    }

    let args = serde_urlencoded::to_string(ValidationArgs { claims, redirect }).unwrap();
    let verification_link = format!("{}verify_auth?{}", base_url, args);

    #[derive(BartDisplay)]
    #[template = "templates/auth-email.html"]
    struct HtmlMail<'a> {
        title: &'a str,
        url: &'a str,
    }

    let email = EmailBuilder::new()
        .to(email)
        .from(sender)
        .subject("Velkommen til magnusogdisa.no 📸")
        .alternative(
            HtmlMail {
                title: "Velkommen til magnusogdisa.no 📸",
                url: &verification_link,
            }
            .to_string(),
            format!("Velkommen 😊\n\nFor å komme til på magnusogdisa.no trenger du bare å følge denne linken:\n\n{}", verification_link),
        )
        .build()
        .unwrap();

    let mut mailer = mailer.lock().expect("Don't know what to do about Poison");
    mailer.send(email.into()).unwrap();
}

impl<S: Spawn + Send + 'static> InitiateAuth<S> {
    async fn issue(
        db_pool: Pool<ConnectionManager<SqliteConnection>>,
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
                maybe_send_email(
                    db_pool, &base_url, email, &claims, mailer, sender, &redirect,
                )
                .await;
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
            self.db_pool,
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

        let title = self.title;

        Ok(Response {
            status: web::Status::Ok,
            representations: vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(
                        crate::site::Layout {
                            title: &title,
                            body: &Post { email: &email },
                        }
                        .to_string(),
                    ) as RepresentationBox
                }) as _,
            )],
            cookies: vec![cookie],
        })
    }

    async fn post_core(self: Box<Self>, content_type: String, body: hyper::Body) -> Response {
        let title = self.title.clone();

        self.try_post(content_type, body)
            .await
            .unwrap_or_else(|e| e.render(&title))
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
