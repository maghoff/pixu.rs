use super::resource::Resource;

pub enum Error {
    BadRequest,
    InternalServerError,
}

pub trait QueryableResource : Send + Sync {
    fn query(self: Box<Self>, query: Option<&str>) -> Result<Box<dyn Resource + Send + Sync>, Error>;
}

impl<T: 'static + Resource + Send + Sync> QueryableResource for T {
    fn query(self: Box<Self>, _query: Option<&str>)
        -> Result<Box<dyn Resource + Send + Sync>, Error>
    {
        Ok(self as _)
    }
}

