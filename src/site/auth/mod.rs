use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};

pub mod authorizer;
mod claims_consumer;
mod initiate_auth;
mod jwt_cookie_handler;
mod verify_auth;

pub use claims_consumer::ClaimsConsumer;
pub use initiate_auth::InitiateAuth;
pub use jwt_cookie_handler::JwtCookieHandler;
pub use verify_auth::VerifyAuthArgsConsumer;

// NumberDate is the name of the type for datetimes in JWT
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
struct NumberDate(i64);

impl From<DateTime<Utc>> for NumberDate {
    fn from(datetime: DateTime<Utc>) -> Self {
        NumberDate(datetime.timestamp())
    }
}

// TODO: Refactor to having ValidationClaims and Claims be a tagged enum?

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthPhase {
    Validation,
    LoggedIn,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidationClaims {
    phase: AuthPhase,
    sub: String,
    exp: NumberDate,
    jti: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub phase: AuthPhase,
    pub sub: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::executor::block_on;
    use web::{CookieHandler, Error, Resource, Response};

    pub struct AuthorizationHandler {
        ok: Resource,
    }

    #[async_trait::async_trait]
    impl ClaimsConsumer for AuthorizationHandler {
        type Claims = Claims;

        async fn claims(self, claims: Option<Self::Claims>) -> Result<Resource, Error> {
            let sub = claims.as_ref().map(|x| x.sub.as_str());

            if sub == Some("let-me-in") {
                Ok(self.ok)
            } else {
                unimplemented!()
            }
        }
    }

    struct Qr;

    #[async_trait::async_trait]
    impl web::Get for Qr {
        async fn representations(self: Box<Self>) -> Response {
            use web::{MediaType, RepresentationBox};
            Response::new(
                web::Status::Ok,
                vec![(
                    MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                    Box::new(move || Box::new("Ok") as RepresentationBox),
                )],
            )
        }
    }

    #[test]
    fn when_successful_then_status_ok() {
        const KEY: &[u8] = b"secret";

        block_on(async {
            use jsonwebtoken::Header;

            let token = jsonwebtoken::encode(
                &Header::default(),
                &Claims {
                    phase: AuthPhase::LoggedIn,
                    sub: "let-me-in".to_owned(),
                },
                KEY,
            )
            .unwrap();
            let token = &[Some(token.as_str())];

            let ok = Resource {
                etag: None,
                get: Some(Box::new(Qr)),
                post: None,
            };
            let c = AuthorizationHandler { ok };
            let a = Box::new(JwtCookieHandler::new(KEY.into(), c));
            let resource = a.cookies(token).await.map_err(|_| ()).unwrap();
            let (Response { status, .. }, _) = resource.get().await;
            assert_eq!(status, web::Status::Ok);
        });
    }
}
