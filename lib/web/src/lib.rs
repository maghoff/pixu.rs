#![feature(async_await, await_macro, futures_api, unsized_locals)]

use core::future::Future;
use futures::future::FutureExt;
use std::pin::Pin;

use hyper::http;
use hyper::{Body, Request, Response};

mod etag;
pub use self::etag::ETag;

mod media_type;
pub use self::media_type::MediaType;

mod representation;
pub use self::representation::Representation;

mod resource;
pub use self::resource::Resource;

mod queryable_resource;
pub use self::queryable_resource::{Error, QueryableResource};

pub trait Lookup: Send {
    fn lookup<'a>(
        &'a self,
        path: &'a str,
    ) -> Pin<Box<dyn Future<Output = Box<dyn QueryableResource>> + Send + 'a>>;
}

enum ResolveError<'a> {
    MalformedUri(&'a http::Uri),
    LookupError(Error),
}

async fn resolve_resource<'a>(
    lookup: &'a (dyn Lookup + 'a + Send + Sync),
    uri: &'a http::Uri,
) -> Result<Box<dyn Resource + Send + 'a>, ResolveError<'a>> {
    match (uri.path(), uri.query()) {
        ("*", None) => unimplemented!("Should return asterisk resource"),
        (path, query) if path.starts_with('/') => {
            let queryable_resource = await!(lookup.lookup(&path[1..]));
            queryable_resource
                .query(query)
                .map_err(ResolveError::LookupError)
        }
        _ => Err(ResolveError::MalformedUri(uri)),
    }
}

fn method_not_allowed() -> (
    http::StatusCode,
    Vec<(
        MediaType,
        Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>,
    )>,
) {
    (
        http::StatusCode::METHOD_NOT_ALLOWED,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || {
                Box::new("Method Not Allowed\n") as Box<dyn Representation + Send + 'static>
            })
                as Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>,
        )],
    )
}

fn bad_request() -> impl Resource {
    (
        http::StatusCode::BAD_REQUEST,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || Box::new("Bad Request\n") as Box<dyn Representation + Send + 'static>)
                as Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>,
        )],
    )
}

fn internal_server_error() -> impl Resource {
    (
        http::StatusCode::INTERNAL_SERVER_ERROR,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || {
                Box::new("Internal Server Error\n") as Box<dyn Representation + Send + 'static>
            })
                as Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>,
        )],
    )
}

async fn handle_request_core<'a>(
    site: &'a (dyn Lookup + 'a + Send + Sync),
    req: Request<Body>,
) -> Response<Body> {
    let (req, body) = req.into_parts();

    let resource: Box<dyn Resource + Send> = await!(resolve_resource(site, &req.uri))
        .unwrap_or_else(|x| match x {
            ResolveError::MalformedUri(_) => Box::new(bad_request()),
            ResolveError::LookupError(_) => Box::new(internal_server_error()),
        });

    let etag = resource.etag();
    let last_modified = resource.last_modified();

    if let Some(_etag) = etag {
        // Check ETag-related If-headers: If-Match, If-None-Match
        // Maybe not contingent on the resource giving an ETag
        unimplemented!();
    }

    if let Some(_last_modified) = last_modified {
        // Grammar reference: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Last-Modified
        // Handle If-Modified-Since and If-Unmodified-Since
        // Maybe not contingent on the resource giving a timestamp
        unimplemented!();
    }

    // let _accept = req.headers().get(http::header::ACCEPT)
    //     .map(|x| x.to_str())
    //     .transpose()
    //     .map_err(|_| Error::BadRequest)?;

    let (status, mut representations) = await!(match req.method {
        // TODO: Implement HEAD and OPTIONS in library
        hyper::Method::GET => async { resource.get() }.boxed(),
        hyper::Method::POST => {
            let content_type = req
                .headers
                .get(http::header::CONTENT_TYPE)
                .map(|x| x.to_str().map(|x| x.to_string())); // TODO should be parsed as a MediaType

            if let Some(Ok(content_type)) = content_type {
                resource.post(content_type, body)
            } else {
                async { Box::new(bad_request()).get() }.boxed()
            }
        }
        _ => async { method_not_allowed() }.boxed() as _,
    });

    let mut response = Response::builder();
    response.status(status);

    // TODO Implement content type negotiation via Accept
    // Also conditionally set Vary: Accept in response
    let (content_type, rep_builder) = representations.pop().unwrap(); // FIXME: Stub
    let representation = rep_builder();

    response.header("content-type", content_type.to_string());

    if let Some(etag) = etag {
        response.header("etag", etag.to_string());
    }

    if let Some(last_modified) = last_modified {
        // See timestamp format: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Last-Modified
        response.header("last-modified", last_modified.to_string());
        unimplemented!("Missing correct datetime formatter");
    }

    // Create response body
    response
        .body(representation.body())
        .expect("Success should be guaranteed at type level")
}

// This exists merely to allow use of .compat() layer for futures 0.1 support
pub async fn handle_request<'a>(
    site: &'a (dyn Lookup + 'a + Send + Sync),
    req: Request<Body>,
) -> Result<Response<Body>, Box<std::error::Error + Send + Sync + 'static>> {
    Ok(await!(handle_request_core(site, req)))
}
