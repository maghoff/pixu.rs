use core::future::Future;
use std::pin::Pin;

use cookie::Cookie;
use futures::future::FutureExt;

use super::etag::ETag;
use super::media_type::MediaType;
use super::representation::Representation;

pub type RepresentationBox = Box<dyn Representation + Send + 'static>;
pub type RendererBox = Box<dyn FnOnce() -> RepresentationBox + Send + 'static>;
pub type RepresentationsVec = Vec<(MediaType, RendererBox)>;
pub type FutureBox<'a, Output> = Pin<Box<dyn Future<Output = Output> + Send + 'a>>;

#[derive(PartialEq, Eq, Debug)]
pub enum Status {
    // 2__
    Ok,
    Created(String),

    // 3__
    MovedPermanently(String),
    SeeOther(String),

    // 4__
    BadRequest,
    Unauthorized, // TODO: `WWW-Authenticate` header
    NotFound,
    MethodNotAllowed, // TODO: `Allow` header

    // 5__
    InternalServerError,
}

pub struct Response {
    pub status: Status,
    pub representations: RepresentationsVec,
    pub cookies: Vec<Cookie<'static>>,
}

impl Response {
    pub fn new(status: Status, representations: RepresentationsVec) -> Response {
        Response {
            status,
            representations,
            cookies: vec![],
        }
    }
}

fn method_not_allowed() -> Response {
    Response::new(
        Status::MethodNotAllowed,
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

    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response>;

    fn post<'a>(
        self: Box<Self>,
        _content_type: String,
        _body: hyper::Body,
    ) -> FutureBox<'a, Response> {
        async { method_not_allowed() }.boxed()
    }
}

impl Resource for (Status, RepresentationsVec) {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        async { Response::new(self.0, self.1) }.boxed()
    }
}

impl Resource for RepresentationsVec {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        async { Response::new(Status::Ok, *self) }.boxed()
    }
}

impl<R: Resource, T: FnOnce() -> Box<R> + Send> Resource for T {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        (*self)().get()
    }
}
