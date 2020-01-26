use super::{Error, Resource};

#[async_trait::async_trait]
pub trait CookieHandler: Send {
    fn read_cookies(&self) -> &[&str];

    // The values are given in the same order as the keys listed by read_cookies()
    async fn cookies(self: Box<Self>, values: &'_ [Option<&'_ str>]) -> Result<Resource, Error>;
}

#[async_trait::async_trait]
impl CookieHandler for Resource {
    fn read_cookies(&self) -> &[&str] {
        &[]
    }

    async fn cookies(self: Box<Self>, _values: &'_ [Option<&'_ str>]) -> Result<Resource, Error> {
        Ok(*self)
    }
}
