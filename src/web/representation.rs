use hyper::Body;

use super::etag::ETag;

pub trait Representation {
    fn etag(&self) -> Option<ETag> {
        None
    }

    fn last_modified(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }

    fn body(self: Box<Self>) -> Body;
}

impl<B: Into<Body>> Representation for B {
    fn body(self: Box<Self>) -> Body {
        (*self).into()
    }
}
