#![feature(async_await, await_macro, futures_api, never_type, transpose_result, unsized_locals)]

#[macro_use] extern crate bart_derive;
#[macro_use] extern crate diesel_migrations;

mod db;

use std::fmt;
use std::net::SocketAddr;

use futures::prelude::*;
use hyper::http;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "pixu.rs")]
struct Options {
    /// SQLite database file
    #[structopt(name = "DB")]
    db: String,
}

use hyper::{Body, Request, Response};

const TEXT_HTML: &str = "text/html;charset=utf-8";

#[derive(BartDisplay)]
#[template_string="You are looking for {{path}}\n"]
struct DummyResponse<'a> {
    path: &'a str,
}

#[derive(BartDisplay)]
#[template_string="Bad request\n"]
struct BadRequest;

#[derive(BartDisplay)]
#[template_string="Internal server error\n"]
struct InternalServerError;

enum Error {
    BadRequest,
    InternalServerError,
}

// String? Really? Maybe Cow or something instead?
enum ETag {
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

type Timestamp = !;

// FIXME Very alloc heavy struct
struct MediaType {
    type_category: String,
    subtype: String,
    args: Vec<String>,
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

trait Representation {
    fn etag(&self) -> Option<ETag>;
    fn last_modified(&self) -> Option<Timestamp>;

    fn body(&self) -> Body;
}

trait Resource {
    fn representations(self: Box<Self>) ->
        Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>;
}

trait QueryableResource {
    fn query(self: Box<Self>, query: Option<&str>) -> Result<Box<dyn Resource>, Error>;
}

struct GreeterResource {
    path: String,
}

impl GreeterResource {
    fn new(path: impl ToString) -> Self {
        Self { path: path.to_string() }
    }
}

impl QueryableResource for GreeterResource {
    fn query(self: Box<Self>, _query: Option<&str>)
        -> Result<Box<dyn Resource>, Error>
    {
        Ok(self as _)
    }
}

impl Resource for GreeterResource {
    fn representations(self: Box<Self>)
        -> Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>
    {
        vec![
            (
                MediaType {
                    type_category: "text".to_string(),
                    subtype: "html".to_string(),
                    args: vec![ "charset=utf-8".to_string() ],
                },
                Box::new(move || {
                    self as Box<dyn Representation>
                }) as _
            )
        ]
    }
}

impl Representation for GreeterResource {
    fn etag(&self) -> Option<ETag> { None }
    fn last_modified(&self) -> Option<Timestamp> { None }

    fn body(&self) -> Body {
        Body::from(DummyResponse { path: &self.path }.to_string())
    }
}

async fn lookup(path: &str) -> Box<dyn QueryableResource> {
    Box::new(GreeterResource::new(path)) as _
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
        response.header("last-modified", last_modified.to_string());
    }

    // Create response body
    Ok(response
        .body(representation.body())
        .expect("Success should be guaranteed at type level"))
}

async fn handle_request(req: Request<Body>) ->
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

fn main() -> Result<(), Box<std::error::Error>>{
    let opt = Options::from_args();
    let _db = db::create_pool(opt.db)?;

    let bind_host = "127.0.0.1".parse().expect("Acceptable IP address");
    let bind_port = 1212;

    let service_fn = || {
        hyper::service::service_fn(
            |req| handle_request(req).boxed().compat()
        )
    };

    let server =
        hyper::server::Server::bind(&SocketAddr::new(bind_host, bind_port))
            .serve(service_fn);

    println!("Listening on http://{}", server.local_addr());

    // The following implicitly starts a thread pool which in turn blocks
    // propagation of panics. I'm not sure I want to deal with panics that
    // way yet.
    //
    // tokio::run(server.map_err(|e| {
    //     eprintln!("server error: {}", e);
    // }));

    // Alternative: Start a tokio core that's limited to the current thread
    use tokio::runtime::current_thread::Runtime;
    let mut runtime = Runtime::new().unwrap();
    runtime.block_on(server).map_err(|e| {
        format!("server error: {}", e)
    })?;

    Ok(())
}
