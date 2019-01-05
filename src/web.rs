use std::fmt;

use hyper::http;
use hyper::{Body, Request, Response};

// FIXME Reverse dependency
// I was unable to dependency inject this async function due to an ICE:
// https://github.com/rust-lang/rust/issues/57084
use crate::site::lookup;

const TEXT_HTML: &str = "text/html;charset=utf-8";

#[derive(BartDisplay)]
#[template_string="Bad request\n"]
struct BadRequest;

#[derive(BartDisplay)]
#[template_string="Internal server error\n"]
struct InternalServerError;

pub enum Error {
    BadRequest,
    InternalServerError,
}

// String? Really? Maybe Cow or something instead?
pub enum ETag {
    Weak(String),
    Strong(String),
}

impl fmt::Display for ETag {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // TODO Escape. Better typing for validating ETags? (IntoETag?)
        // Reference for ETag grammar: https://stackoverflow.com/a/11572348
        match self {
            ETag::Weak(tag) => write!(fmt, "W/\"{}\"", tag),
            ETag::Strong(tag) => write!(fmt, "\"{}\"", tag),
        }
    }
}

// FIXME Very alloc heavy struct
// FIXME Verify validity of data on creation
pub struct MediaType {
    pub type_category: String,
    pub subtype: String,
    pub args: Vec<String>,
}

impl fmt::Display for MediaType {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // FIXME: Will willingly generate invalid media type strings if the
        // components are invalid

        write!(fmt, "{}/{}", self.type_category, self.subtype)?;

        for (i, arg) in self.args.iter().enumerate() {
            if i == 0 {
                write!(fmt, ";")?;
            } else {
                write!(fmt, "&")?;
            }
            write!(fmt, "{}", arg)?;
        }

        Ok(())
    }
}

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

pub trait Resource {
    fn representations(self: Box<Self>) ->
        Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>;
}

pub trait QueryableResource {
    fn query(self: Box<Self>, query: Option<&str>) -> Result<Box<dyn Resource>, Error>;
}

impl<T: 'static + Resource> QueryableResource for T {
    fn query(self: Box<Self>, _query: Option<&str>)
        -> Result<Box<dyn Resource>, Error>
    {
        Ok(self as _)
    }
}

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

async fn handle_request_core(req: Request<Body>) ->
    Result<Response<Body>, Error>
{
    let resource = await!(resolve_resource(&req.uri())).map_err(|x| match x {
        ResolveError::MalformedUri(_) => Error::BadRequest,
        ResolveError::LookupError(_) => Error::InternalServerError,
    })?;

    let _accept = req.headers().get(http::header::ACCEPT)
        .map(|x| x.to_str())
        .transpose()
        .map_err(|_| Error::BadRequest)?;

    let mut representations = resource.representations();

    // TODO Implement content type negotiation via Accept
    // Also conditionally set Vary: Accept in response
    let (content_type, rep_builder) = representations.pop().unwrap(); // FIXME: Stub
    let representation = rep_builder();

    let etag = representation.etag();
    let last_modified = representation.last_modified();

    if let Some(_etag) = etag {
        // Check ETag-related If-headers: If-Match, If-None-Match
        // Maybe not contingent on the representation giving an ETag
        unimplemented!();
    }

    if let Some(_last_modified) = representation.last_modified() {
        // Grammar reference: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Last-Modified
        // Handle If-Modified-Since and If-Unmodified-Since
        // Maybe not contingent on the representation giving a timestamp
        unimplemented!();
    }

    let mut response = Response::builder();
    response.status(http::StatusCode::OK);
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
