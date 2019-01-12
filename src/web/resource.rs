use hyper::http;

use super::etag::ETag;
use super::media_type::MediaType;
use super::representation::Representation;

pub trait Resource {
    fn etag(&self) -> Option<ETag> {
        None
    }

    fn last_modified(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }

    fn get(self: Box<Self>) ->
        (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>);
}

impl Resource for (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>) {
    fn get(self: Box<Self>)
        -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>)
    {
        *self
    }
}

impl Resource for Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)> {
    fn get(self: Box<Self>)
        -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>)
    {
        (http::StatusCode::OK, *self)
    }
}

impl<R: Resource, T: FnOnce() -> Box<R>> Resource for T {
    fn get(self: Box<Self>)
        -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>)
    {
        (*self)().get()
    }
}
