use hyper::http;
use hyper::{Body, Request, Response};
use insideout::InsideOut;

// FIXME Reverse dependency
// I was unable to dependency inject this async function due to an ICE:
// https://github.com/rust-lang/rust/issues/57084
use crate::site::lookup;

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

const TEXT_HTML: &str = "text/html;charset=utf-8";

#[derive(BartDisplay)]
#[template_string="Bad request\n"]
struct BadRequest;

#[derive(BartDisplay)]
#[template_string="Internal server error\n"]
struct InternalServerError;

enum ResolveError<'a> {
    MalformedUri(&'a http::Uri),
    LookupError(Error),
}

async fn resolve_resource(uri: &http::Uri) -> Result<Box<dyn Resource>, ResolveError> {
    match (uri.path(), uri.query()) {
        ("*", None) => unimplemented!("Should return asterisk resource"),
        (path, query) if path.starts_with('/') => {
            let queryable_resource = await!(lookup(&path[1..]));
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

async fn handle_request_core(req: Request<Body>) ->
    Result<Response<Body>, Error>
{
    let resource = await!(resolve_resource(&req.uri())).map_err(|x| match x {
        // Change to follow through with "special" resource instances?
        // Like method_not_allowed() below. This change would make the
        // function not return a Result at all..! Funny. And good?
        ResolveError::MalformedUri(_) => Error::BadRequest,
        ResolveError::LookupError(_) => Error::InternalServerError,
    })?;

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

    let _accept = req.headers().get(http::header::ACCEPT)
        .map(|x| x.to_str())
        .inside_out()
        .map_err(|_| Error::BadRequest)?;

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
    Ok(response
        .body(representation.body())
        .expect("Success should be guaranteed at type level"))
}

pub async fn handle_request(req: Request<Body>) ->
    Result<Response<Body>, Box<std::error::Error + Send + Sync + 'static>>
{
    match await!(handle_request_core(req)) {
        Ok(res) => Ok(res),
        Err(Error::BadRequest) => {
            let body = BadRequest;

            Ok(Response::builder()
                .status(http::StatusCode::BAD_REQUEST)
                .header(http::header::CONTENT_TYPE, TEXT_HTML)
                .body(Body::from(body.to_string()))
                .unwrap()
            )
        },
        Err(Error::InternalServerError) => {
            let body = InternalServerError;

            Ok(Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .header(http::header::CONTENT_TYPE, TEXT_HTML)
                .body(Body::from(body.to_string()))
                .unwrap()
            )
        },
    }
}
