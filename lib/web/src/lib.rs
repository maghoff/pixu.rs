#![feature(async_await, await_macro, futures_api, unsized_locals)]

use futures::future::FutureExt;

use hyper::http;
use hyper::{Body, Request, Response};

mod etag;
mod media_type;
mod queryable_resource;
mod representation;
mod resource;

pub use self::etag::ETag;
pub use self::media_type::MediaType;
pub use self::queryable_resource::{Error, QueryableResource};
pub use self::representation::Representation;
pub use self::resource::*;

pub trait Lookup: Send {
    fn lookup<'a>(&'a self, path: &'a str) -> FutureBox<'a, Box<dyn QueryableResource>>;
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

fn method_not_allowed() -> (http::StatusCode, RepresentationsVec) {
    (
        http::StatusCode::METHOD_NOT_ALLOWED,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || Box::new("Method Not Allowed\n") as RepresentationBox) as _,
        )],
    )
}

fn bad_request() -> impl Resource {
    (
        http::StatusCode::BAD_REQUEST,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || Box::new("Bad Request\n") as RepresentationBox) as _,
        )],
    )
}

fn internal_server_error() -> impl Resource {
    (
        http::StatusCode::INTERNAL_SERVER_ERROR,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || Box::new("Internal Server Error\n") as RepresentationBox) as _,
        )],
    )
}

trait HeaderMapExt {
    // Yields Err(Error::BadRequest) when header is present with non-ASCII data
    fn get_ascii(&self, name: http::header::HeaderName) -> Result<Option<&str>, Error>;
}

impl HeaderMapExt for http::HeaderMap<http::header::HeaderValue> {
    fn get_ascii(&self, name: http::header::HeaderName) -> Result<Option<&str>, Error> {
        self.get(name)
            .map(|x| x.to_str()) // Validates that the given data is ASCII
            .transpose()
            .map_err(|_| Error::BadRequest)
    }
}

fn parse_cookie_header(
    src: &str,
) -> impl Iterator<Item = Result<(&str, &str), cookie::ParseError>> {
    src.split("; ").map(|raw| {
        cookie::Cookie::parse(raw).map(|c| (c.name_raw().unwrap(), c.value_raw().unwrap()))
    })
}

async fn try_handle_request<'a>(
    site: &'a (dyn Lookup + 'a + Send + Sync),
    req: Request<Body>,
) -> Result<(Option<ETag>, http::StatusCode, RepresentationsVec), Error> {
    let (req, body) = req.into_parts();

    let mut resource: Box<dyn Resource + Send> = await!(resolve_resource(site, &req.uri))
        .unwrap_or_else(|x| match x {
            ResolveError::MalformedUri(_) => Box::new(bad_request()),
            ResolveError::LookupError(_) => Box::new(internal_server_error()),
        });

    let read_cookies = resource.read_cookies();
    if read_cookies.len() > 0 {
        // TODO Set Vary: Cookie

        let mut cookies = vec![None; read_cookies.len()];

        let cookie_header = req.headers.get_ascii(http::header::COOKIE)?;

        if let Some(cookie_header) = cookie_header {
            for cookie in parse_cookie_header(cookie_header) {
                let (key, value) = cookie.map_err(|_| Error::BadRequest)?;
                let index = read_cookies.iter().position(|&given| given == key);
                if let Some(index) = index {
                    cookies[index] = Some(value);
                }
            }
        }

        resource.cookies(&cookies)?;
    }

    let etag = resource.etag();

    if let Some(_if_match) = req.headers.get_ascii(http::header::IF_MATCH)? {
        unimplemented!();

        /*
        https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-Match

        If none of the given ETags match the etag from the resource, return 412 Precondition Failed
        (Has interesting combination with Range-requests)
        In case of no ETag on resource, always 412
        */
    }

    if let Some(_if_none_match) = req.headers.get_ascii(http::header::IF_NONE_MATCH)? {
        unimplemented!();

        /*
        https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-None-Match

        If any of the given ETags match the etag from the resource and the verb is
            GET, HEAD => 304 Not Modified
            PUT, POST, DELETE => 412 Precondition Failed
            OPTIONS => Uh..?

        Note that the server generating a 304 response MUST generate any of the following header
        fields that would have been sent in a 200 (OK) response to the same request: Cache-Control,
        Content-Location, Date, ETag, Expires, and Vary.
        */
    }

    let _accept = req.headers.get_ascii(http::header::ACCEPT)?;

    let (status, representations) = await!(match req.method {
        // TODO: Implement HEAD and OPTIONS in library
        hyper::Method::GET => resource.get(),
        hyper::Method::POST => {
            let content_type = req
                .headers
                .get(http::header::CONTENT_TYPE)
                .map(|x| x.to_str().map(|x| x.to_string())); // TODO should be parsed as a MediaType

            if let Some(Ok(content_type)) = content_type {
                resource.post(content_type, body)
            } else {
                Box::new(bad_request()).get()
            }
        }
        _ => async { method_not_allowed() }.boxed() as _,
    });

    Ok((etag, status, representations))
}

use hyper::http::StatusCode;

async fn build_response(
    etag: Option<ETag>,
    status: StatusCode,
    mut representations: RepresentationsVec,
) -> Response<Body> {
    let mut response = Response::builder();
    response.status(status);

    if representations.len() > 1 {
        response.header("vary", "accept");
    }

    // Implement content type negotiation via Accept
    let (content_type, rep_builder) = representations.pop().unwrap(); // FIXME: Stub
    let representation = rep_builder();

    response.header("content-type", content_type.to_string());

    if let Some(etag) = etag {
        response.header("etag", etag.to_string());
    }

    // Optionally set Cache-Control

    response
        .body(representation.body())
        .expect("Success should be guaranteed at type level")
}

async fn handle_request_core<'a>(
    site: &'a (dyn Lookup + 'a + Send + Sync),
    req: Request<Body>,
) -> Response<Body> {
    let (etag, status, representations) =
        await!(try_handle_request(site, req)).unwrap_or_else(|err| match err {
            Error::BadRequest => unimplemented!(),
            Error::InternalServerError => unimplemented!(),
        });

    await!(build_response(etag, status, representations))
}

// This exists merely to allow use of .compat() layer for futures 0.1 support
pub async fn handle_request<'a, L>(
    site: std::sync::Arc<L>,
    req: Request<Body>,
) -> Result<Response<Body>, Box<std::error::Error + Send + Sync + 'static>>
where
    L: Lookup + 'a + Send + Sync,
{
    Ok(await!(handle_request_core(&*site, req)))
}
