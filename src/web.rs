use std::fmt;

use hyper::http;
use hyper::{Body, Request, Response};

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

pub type Timestamp = !;

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
    fn etag(&self) -> Option<ETag>;
    fn last_modified(&self) -> Option<Timestamp>;

    fn body(&self) -> Body;
}

pub trait Resource {
    fn representations(self: Box<Self>) ->
        Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>;
}

pub trait QueryableResource {
    fn query(self: Box<Self>, query: Option<&str>) -> Result<Box<dyn Resource>, Error>;
}

enum ResolveError<'a> {
    MalformedUri(&'a http::Uri),
    LookupError(Error),
}

async fn resolve_resource<'a, F, Fut>(f: F, s: &'a str)
    -> Result<(), ()>
where
    F: FnOnce(&'a str) -> Fut,
    F: 'a,
    Fut: futures::Future
{
    let _ = await!(f(s));
    unimplemented!()
}

async fn l(_: &str) {}

async fn handle_request_core(req: Request<Body>) ->
    Result<Response<Body>, Error>
{
    let resource = await!(resolve_resource(crate::site::lookup, "lol"))
        .map(|_| -> Box<dyn Resource> { unimplemented!() })
        .map_err(|_| ResolveError::MalformedUri(&req.uri()))
        .map_err(|x| match x {
        ResolveError::MalformedUri(_) => Error::BadRequest,
        ResolveError::LookupError(_) => Error::InternalServerError,
    })?;

    let resource: Box<dyn Resource> = unimplemented!();

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
        response.header("last-modified", last_modified.to_string());
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
