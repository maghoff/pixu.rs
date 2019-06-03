use futures::future::FutureExt;
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};

use web::{CookieHandler, Error, FutureBox, Resource};

pub struct JwtCookieHandler<Consumer, Claims>
where
    Consumer: ClaimsConsumer<Claims = Claims>,
    Claims: DeserializeOwned,
{
    consumer: Consumer,
}

impl<Consumer, Claims> JwtCookieHandler<Consumer, Claims>
where
    Consumer: ClaimsConsumer<Claims = Claims> + Send,
    Claims: DeserializeOwned,
{
    pub fn new(consumer: Consumer) -> Self {
        JwtCookieHandler { consumer }
    }

    async fn cookies_async<'a>(
        self: Box<Self>,
        values: &'a [Option<&'a str>],
    ) -> Result<Box<dyn Resource + Send + 'static>, Error> {
        let token_data = if let Some(jwt) = values[0] {
            use jsonwebtoken::{Algorithm, Validation};

            jsonwebtoken::decode::<Claims>(
                jwt,
                "secret".as_ref(),
                &Validation {
                    algorithms: vec![Algorithm::HS256],
                    validate_exp: false,
                    ..Default::default()
                },
            )
            .unwrap_or_else(|_| unimplemented!())
        } else {
            unimplemented!()
        };

        self.consumer.authorization(token_data.claims).await
    }
}

impl<Consumer, Claims> CookieHandler for JwtCookieHandler<Consumer, Claims>
where
    Consumer: 'static + ClaimsConsumer<Claims = Claims> + Send,
    Claims: 'static + DeserializeOwned + Send,
{
    fn read_cookies(&self) -> &[&str] {
        &["let-me-in"]
    }

    fn cookies<'a>(
        self: Box<Self>,
        values: &'a [Option<&'a str>],
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>> {
        self.cookies_async(values).boxed()
    }
}

pub trait ClaimsConsumer {
    type Claims;

    fn authorization<'a>(
        self,
        claims: Self::Claims,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: String,
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

    fn authorization<'a>(
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
