use futures::future::FutureExt;
use serde_derive::{Deserialize, Serialize};

use web::{Error, FutureBox, Resource};

mod claims_consumer;
mod jwt_cookie_handler;

pub use claims_consumer::ClaimsConsumer;
pub use jwt_cookie_handler::JwtCookieHandler;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
}

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
        claims: Self::Claims,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>> {
        if claims.sub == "let-me-in" {
            async { Ok(Box::new(self.ok) as Box<dyn Resource + Send + 'static>) }.boxed() as _
        } else {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::executor::block_on;
    use web::CookieHandler;

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
            let (status, _) = resource.get().await;
            assert_eq!(status, 200);
        });
    }
}
