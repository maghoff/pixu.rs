use futures::future::FutureExt;

use super::Resource;
use super::queryable_resource::Error;
use super::FutureBox;

pub trait CookieHandler: Send {
    fn read_cookies(&self) -> &[&str];

    // The values are given in the same order as the keys listed by read_cookies()
    fn cookies<'a>(
        self: Box<Self>,
        _values: &'a [Option<&'a str>],
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>>;
}

impl<T> CookieHandler for T
where
    T: Resource + Send + 'static,
{
    fn read_cookies(&self) -> &[&str] {
        &[]
    }

    fn cookies<'a>(
        self: Box<Self>,
        _values: &'a [Option<&'a str>],
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>> {
        async { Ok(self as _) }.boxed()
    }
}

