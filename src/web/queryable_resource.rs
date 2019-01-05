use super::resource::Resource;

pub enum Error {
    BadRequest,
    InternalServerError,
}

pub trait QueryableResource {
    fn query(self: Box<Self>, query: Option<&str>) -> Result<Box<dyn Resource>, Error>;
}

impl<T: 'static + Resource> QueryableResource for T {
    fn query(self: Box<Self>, _query: Option<&str>)
        -> Result<Box<dyn Resource>, Error>
    {
        Ok(self as _)
    }
}

