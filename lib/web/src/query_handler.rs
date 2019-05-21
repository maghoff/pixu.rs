use super::CookieHandler;

pub enum Error {
    BadRequest,
    InternalServerError,
}

pub trait QueryHandler: Send {
    fn query(self: Box<Self>, query: Option<&str>) -> Result<Box<dyn CookieHandler + Send>, Error>;
}

impl<T: 'static + CookieHandler + Send> QueryHandler for T {
    fn query(
        self: Box<Self>,
        _query: Option<&str>,
    ) -> Result<Box<dyn CookieHandler + Send>, Error> {
        Ok(self as _)
    }
}
