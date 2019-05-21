use futures::future::FutureExt;
use futures::Future;

use web::{CookieHandler, Error, FutureBox, Resource};

pub struct AuthorizationProvider<Fun, Res, Fut>
where
    Fun: 'static + FnOnce() -> Fut + Send,
    Res: 'static + Resource,
    Fut: 'static + Future<Output = Res> + Send,
{
    create_inner: Fun,
}

impl<Fun, Res, Fut> AuthorizationProvider<Fun, Res, Fut>
where
    Fun: 'static + FnOnce() -> Fut + Send,
    Res: 'static + Resource,
    Fut: 'static + Future<Output = Res> + Send,
{
    pub fn new(create_inner: Fun) -> Self {
        AuthorizationProvider { create_inner }
    }

    async fn cookies_async<'a>(
        self: Box<Self>,
        values: &'a [Option<&'a str>],
    ) -> Result<Box<dyn Resource + Send + 'static>, Error> {
        let let_me_in = values[0] == Some("yes");

        if let_me_in {
            Ok(Box::new(await!((self.create_inner)())) as _)
        } else {
            unimplemented!()
        }
    }
}

impl<Fun, Res, Fut> CookieHandler for AuthorizationProvider<Fun, Res, Fut>
where
    Fun: 'static + FnOnce() -> Fut + Send,
    Res: 'static + Resource,
    Fut: 'static + Future<Output = Res> + Send,
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

#[cfg(test)]
mod test {
    use super::*;
    use web::{MediaType, RepresentationBox};

    async fn qr() -> impl Resource {

        vec![(
            MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || Box::new("Ok") as RepresentationBox) as _,
        )]
    }

    #[test]
    fn one() {
        let _a = AuthorizationProvider::new(qr);
    }
}
