use futures::future::FutureExt;

use web::{CookieHandler, Error, FutureBox, Resource};

pub struct AuthorizationProvider<Consumer>
where
    Consumer: AuthorizationConsumer,
{
    consumer: Consumer,
}

impl<Consumer> AuthorizationProvider<Consumer>
where
    Consumer: AuthorizationConsumer<Authorization = bool> + Send,
{
    pub fn new(consumer: Consumer) -> Self {
        AuthorizationProvider { consumer }
    }

    async fn cookies_async<'a>(
        self: Box<Self>,
        values: &'a [Option<&'a str>],
    ) -> Result<Box<dyn Resource + Send + 'static>, Error> {
        let let_me_in = values[0] == Some("yes");
        // TODO Decode JWT instead

        self.consumer.authorization(let_me_in).await
    }
}

impl<Consumer> CookieHandler for AuthorizationProvider<Consumer>
where
    Consumer: 'static + AuthorizationConsumer<Authorization = bool> + Send,
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

pub trait AuthorizationConsumer {
    // Forward cookies?

    type Authorization;

    fn authorization<'a>(
        self,
        authorization: Self::Authorization,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>>;
}

pub struct SimpleBoolAuthConsumer<R: Resource> {
    ok: R,
}

impl<R: Resource> SimpleBoolAuthConsumer<R> {
    pub fn new(ok: R) -> SimpleBoolAuthConsumer<R> {
        SimpleBoolAuthConsumer { ok }
    }
}

impl<R: 'static + Resource> AuthorizationConsumer for SimpleBoolAuthConsumer<R> {
    type Authorization = bool;

    fn authorization<'a>(
        self,
        authorized: Self::Authorization,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>> {
        if authorized {
            async { Ok(Box::new(self.ok) as Box<dyn Resource + Send + 'static>) }.boxed() as _
        } else {
            unimplemented!()
        }
    }
}

/*

AuthorizationProvider --AuthorizationData--> AuthorizationConsumer -> Resource

*/

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
            let c = SimpleBoolAuthConsumer::new(qr().await);
            let a = Box::new(AuthorizationProvider::new(c));
            let resource = a.cookies(&[Some("yes")]).await.unwrap();
            let (status, _) = resource.get().await;
            assert_eq!(status, 200);
        });
    }
}
