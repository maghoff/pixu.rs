use futures::FutureExt;
use jsonwebtoken::{encode, Algorithm, Header, Validation};
use serde_derive::Deserialize;
use web::{Cookie, CookieHandler, FutureBox, MediaType, RepresentationBox, Resource, Response};

use super::{AuthPhase, Claims, ValidationClaims};
use crate::site::handling_error::HandlingError;
use crate::site::query_args::QueryArgsConsumer;

pub struct VerifyAuth {
    key: Vec<u8>,
    claims: String,
    head_sign: String,
    redirect: String,
}

#[derive(BartDisplay)]
#[template = "templates/verify-auth.html"]
struct Get;

#[derive(Deserialize)]
pub struct ValidationArgsOwned {
    claims: String,
    redirect: String,
}

fn verify_login(
    key: &[u8],
    head_sign: &str,
    claims: &str,
) -> Result<ValidationClaims, Box<dyn std::error::Error>> {
    let mut head_sign = head_sign.splitn(2, '.');
    let head = head_sign.next().unwrap();
    let sign = head_sign.next().ok_or("Missing . in head_sign")?;

    let token = format!("{}.{}.{}", head, claims, sign);

    let token = jsonwebtoken::decode::<ValidationClaims>(
        &token,
        key,
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

impl VerifyAuth {
    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        let claims = verify_login(&self.key, &self.head_sign, &self.claims)
            .map_err(|_| HandlingError::BadRequest("Unable to verify login"))?;

        let claims = Claims {
            phase: AuthPhase::LoggedIn,
            sub: claims.sub,
        };

        let token = encode(&Header::default(), &claims, &self.key).unwrap();
        let cookie = Cookie::build("let-me-in", token).http_only(true).finish();

        Ok(Response {
            status: web::Status::SeeOther(self.redirect),
            representations: vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || Box::new(Get.to_string()) as RepresentationBox) as _,
            )],
            cookies: vec![cookie],
        })
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
    key: Vec<u8>,
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
            key: self.key,
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

pub struct VerifyAuthArgsConsumer {
    pub key: Vec<u8>,
}

impl QueryArgsConsumer for VerifyAuthArgsConsumer {
    type Args = ValidationArgsOwned;

    fn args(self, args: Self::Args) -> Result<Box<dyn web::CookieHandler + Send>, web::Error> {
        Ok(Box::new(VerifyAuthCookieHandler {
            key: self.key,
            claims: args.claims,
            redirect: args.redirect,
        }))
    }
}
