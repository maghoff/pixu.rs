use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};

mod claims_consumer;
mod initiate_auth;
mod jwt_cookie_handler;
mod verify_auth;

pub use claims_consumer::ClaimsConsumer;
pub use initiate_auth::InitiateAuth;
pub use jwt_cookie_handler::JwtCookieHandler;
pub use verify_auth::VerifyAuthArgsConsumer;

// TODO Oi! Global state!
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::executor::block_on;
    use futures::future::FutureExt;
    use web::{CookieHandler, Error, FutureBox, Resource, Response};

    pub struct AuthorizationHandler<R: Resource> {
        ok: R,
    }

    impl<R: Resource> AuthorizationHandler<R> {
        pub fn new(ok: R) -> AuthorizationHandler<R> {
            AuthorizationHandler { ok }
        }
    }

    impl<R: 'static + Resource> ClaimsConsumer for AuthorizationHandler<R> {
        type Claims = Claims;

        fn claims<'a>(
            self,
            claims: Option<Self::Claims>,
        ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>> {
            let sub = claims.as_ref().map(|x| x.sub.as_str());

            if sub == Some("let-me-in") {
                async { Ok(Box::new(self.ok) as Box<dyn Resource + Send + 'static>) }.boxed() as _
            } else {
                unimplemented!()
            }
        }
    }

    async fn qr() -> impl Resource {
        use web::{MediaType, RepresentationBox};
        vec![(
            MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || Box::new("Ok") as RepresentationBox) as _,
        )]
    }

    #[test]
    fn when_successful_then_status_ok() {
        block_on(async {
            use jsonwebtoken::Header;

            let token = jsonwebtoken::encode(
                &Header::default(),
                &Claims {
                    sub: "let-me-in".to_owned(),
                },
                "secret".as_ref(),
            )
            .unwrap();
            let token = &[Some(token.as_str())];

            let c = AuthorizationHandler::new(qr().await);
            let a = Box::new(JwtCookieHandler::new(c));
            let resource = a.cookies(token).await.unwrap();
            let Response { status, .. } = resource.get().await;
            assert_eq!(status, 200);
        });
    }
}
