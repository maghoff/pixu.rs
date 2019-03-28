use super::resource::Resource;

pub enum Error {
    BadRequest,
    InternalServerError,
}

pub trait QueryableResource : Send {
    fn query(self: Box<Self>, query: Option<&str>) -> Result<Box<dyn Resource + Send>, Error>;
}

impl<T: 'static + Resource + Send> QueryableResource for T {
    fn query(self: Box<Self>, _query: Option<&str>)
        -> Result<Box<dyn Resource + Send>, Error>
    {
        Ok(self as _)
    }
}

