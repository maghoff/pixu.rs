use core::future::Future;
use std::pin::Pin;

use futures::future::FutureExt;
use hyper::http;

use super::etag::ETag;
use super::media_type::MediaType;
use super::representation::Representation;

pub type RepresentationBox = Box<dyn Representation + Send + 'static>;
pub type RendererBox = Box<dyn FnOnce() -> RepresentationBox + Send + 'static>;
pub type RepresentationsVec = Vec<(MediaType, RendererBox)>;
pub type FutureBox<'a, Output> = Pin<Box<dyn Future<Output = Output> + Send + 'a>>;

fn method_not_allowed() -> (http::StatusCode, RepresentationsVec) {
    (
        http::StatusCode::METHOD_NOT_ALLOWED,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || Box::new("Method Not Allowed\n") as RepresentationBox) as _,
        )],
    )
}

pub trait Resource: Send {
    fn etag(&self) -> Option<ETag> {
        None
    }

    fn last_modified(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }

    fn get(self: Box<Self>) -> (http::StatusCode, RepresentationsVec);

    fn post<'a>(
        self: Box<Self>,
        _content_type: String,
        _body: hyper::Body,
    ) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        async { method_not_allowed() }.boxed()
    }
}

impl Resource for (http::StatusCode, RepresentationsVec) {
    fn get(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        *self
    }
}

impl Resource for RepresentationsVec {
    fn get(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        (http::StatusCode::OK, *self)
    }
}

impl<R: Resource, T: FnOnce() -> Box<R> + Send> Resource for T {
    fn get(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        (*self)().get()
    }
}
