use futures::future::FutureExt;
use serde::de::DeserializeOwned;

use web::{CookieHandler, Error, FutureBox, Resource};

use super::ClaimsConsumer;

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
        let token_data = values[0].and_then(|jwt| {
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
            .map(|x| x.claims) // Invalid JWTs are ignored
            .ok()
        });

        self.consumer.claims(token_data).await
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
