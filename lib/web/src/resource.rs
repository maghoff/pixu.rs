use core::future::Future;
use std::pin::Pin;

use futures::future::FutureExt;
use hyper::http;

use super::etag::ETag;
use super::media_type::MediaType;
use super::representation::Representation;
use super::queryable_resource::Error;

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
    // TODO Consider moving read_cookies and cookies to a separate trait and
    // let fn cookies consume Box<Self> and return Box<dyn Resource>
    fn read_cookies(&self) -> &[&str] {
        &[]
    }

    // The values are given in the same order as the keys listed by read_cookies()
    fn cookies(&mut self, _values: &[Option<&str>]) -> Result<(), Error> { Ok(()) }

    fn etag(&self) -> Option<ETag> {
        None
    }

    fn get<'a>(self: Box<Self>) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)>;

    fn post<'a>(
        self: Box<Self>,
        _content_type: String,
        _body: hyper::Body,
    ) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        async { method_not_allowed() }.boxed()
    }
}

impl Resource for (http::StatusCode, RepresentationsVec) {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        async { *self }.boxed()
    }
}

impl Resource for RepresentationsVec {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        async { (http::StatusCode::OK, *self) }.boxed()
    }
}

impl<R: Resource, T: FnOnce() -> Box<R> + Send> Resource for T {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        (*self)().get()
    }
}
