#![feature(async_await, await_macro, futures_api, pin, unsized_locals)]

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

pub trait Lookup : Sync + Send {
    fn lookup<'a>(&'a self, path: &'a str) ->
        Pin<Box<dyn core::future::Future<Output=Box<dyn QueryableResource>> + Send + Sync + 'a>>;
}

enum ResolveError<'a> {
    MalformedUri(&'a http::Uri),
    LookupError(Error),
}

async fn resolve_resource<'a>(lookup: &'a (dyn Lookup + 'a + Send + Sync), uri: &'a http::Uri) -> Result<Box<dyn Resource + 'a>, ResolveError<'a>> {
    match (uri.path(), uri.query()) {
        ("*", None) => unimplemented!("Should return asterisk resource"),
        (path, query) if path.starts_with('/') => {
            let queryable_resource = await!(lookup.lookup(&path[1..]));
            queryable_resource.query(query)
                .map_err(ResolveError::LookupError)
        },
        _ => Err(ResolveError::MalformedUri(uri)),
    }
}

fn method_not_allowed() -> (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>) {
    (
        http::StatusCode::METHOD_NOT_ALLOWED,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || {
                Box::new("Method Not Allowed\n") as Box<dyn Representation>
            }) as Box<dyn FnOnce() -> Box<dyn Representation>>
        )]
    )
}

fn bad_request() -> impl Resource {
    (
        http::StatusCode::BAD_REQUEST,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || {
                Box::new("Bad Request\n") as Box<dyn Representation>
            }) as Box<dyn FnOnce() -> Box<dyn Representation>>
        )]
    )
}

fn internal_server_error() -> impl Resource {
    (
        http::StatusCode::INTERNAL_SERVER_ERROR,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || {
                Box::new("Internal Server Error\n") as Box<dyn Representation>
            }) as Box<dyn FnOnce() -> Box<dyn Representation>>
        )]
    )
}

async fn handle_request_core<'a>(site: &'a (dyn Lookup + 'a + Send + Sync), req: Request<Body>) -> Response<Body>
{
    let resource = await!(resolve_resource(site, &req.uri())).unwrap_or_else(|x| match x {
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
    //     .inside_out()
    //     .map_err(|_| Error::BadRequest)?;

    let (status, mut representations) = match req.method() {
        &hyper::Method::GET => resource.get(),
        _ => method_not_allowed(),
    };

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
pub async fn handle_request<'a>(site: &'a (dyn Lookup + 'a + Send + Sync), req: Request<Body>) ->
    Result<Response<Body>, Box<std::error::Error + Send + Sync + 'static>>
{
    Ok(await!(handle_request_core(site, req)))
}
