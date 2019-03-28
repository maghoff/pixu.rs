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

pub trait Resource : Send {
    fn etag(&self) -> Option<ETag> {
        None
    }

    fn last_modified(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }

    fn get(self: Box<Self>) ->
        (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>);

    fn post<'a>(self: Box<Self>, _body: hyper::Body) ->
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

impl Resource for (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>) {
    fn get(self: Box<Self>)
        -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>)
    {
        let (media_type, representations) = *self;
        let representations = unsafe {
            // TODO Revisit. Can we make do without unsafe?
            // https://stackoverflow.com/a/55081958/2971
            let v = Vec::from_raw_parts(
                representations.as_ptr() as _,
                representations.len(),
                representations.capacity()
            );
            std::mem::forget(representations);
            v
        };

        (media_type, representations)
    }
}

impl Resource for Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)> {
    fn get(self: Box<Self>)
        -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>)
    {
        let representations = *self;
        let representations = unsafe {
            // TODO Revisit. Can we make do without unsafe?
            // https://stackoverflow.com/a/55081958/2971
            let v = Vec::from_raw_parts(
                representations.as_ptr() as _,
                representations.len(),
                representations.capacity()
            );
            std::mem::forget(representations);
            v
        };

        (http::StatusCode::OK, representations)
    }
}

impl<R: Resource, T: FnOnce() -> Box<R> + Send> Resource for T {
    fn get(self: Box<Self>)
        -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>)>)
    {
        (*self)().get()
    }
}
