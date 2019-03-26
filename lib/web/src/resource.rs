use core::future::Future;
use std::pin::Pin;

use futures::future::FutureExt;
use hyper::http;

use super::etag::ETag;
use super::media_type::MediaType;
use super::representation::Representation;

fn method_not_allowed() -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>) {
    (
        http::StatusCode::METHOD_NOT_ALLOWED,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || {
                Box::new("Method Not Allowed\n") as Box<dyn Representation + Send + 'static>
            }) as Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>
        )]
    )
}

pub trait Resource : Sync + Send {
    fn etag(&self) -> Option<ETag> {
        None
    }

    fn last_modified(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }

    fn get(self: Box<Self>) ->
        (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>);

    fn post<'a>(self: Box<Self>) ->
        Pin<Box<
            dyn Future<
                Output=(
                    http::StatusCode,
                    Vec<(
                        MediaType,
                        Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>
                    )>
                )
            > + Send + 'a
        >>
    {
        async { method_not_allowed() }.boxed()
    }
}

impl Resource for (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + Sync + 'static>)>) {
    fn get(self: Box<Self>)
        -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>)
    {
        unimplemented!() //*self
    }
}

impl Resource for Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + Sync + 'static>)> {
    fn get(self: Box<Self>)
        -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>)
    {
        unimplemented!() //(http::StatusCode::OK, *self)
    }
}

impl<R: Resource, T: FnOnce() -> Box<R> + Send + Sync> Resource for T {
    fn get(self: Box<Self>)
        -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>)
    {
        (*self)().get()
    }
}
