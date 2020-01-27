#![feature(unsized_locals)]

use std::fmt::Write;

pub use cookie::Cookie;
use hyper::http;
use hyper::{Body, Request};

mod cookie_handler;
mod etag;
mod media_type;
mod query_handler;
mod representation;
mod resource;

pub use self::cookie_handler::CookieHandler;
pub use self::etag::ETag;
pub use self::media_type::MediaType;
pub use self::query_handler::{Error, QueryHandler};
pub use self::representation::Representation;
pub use self::resource::*;

#[async_trait::async_trait]
pub trait Lookup: Send {
    async fn lookup(&'_ self, path: &'_ str) -> Result<Box<dyn QueryHandler>, Response>;
}

enum ResolveError<'a> {
    MalformedUri(&'a http::Uri),
    LookupError(Error),
    GoodError(Response), // TODO Make this the only variant, remove enum
}

async fn resolve_resource<'a>(
    lookup: &'a (dyn Lookup + 'a + Send + Sync),
    uri: &'a http::Uri,
) -> Result<Box<dyn CookieHandler + Send + 'a>, ResolveError<'a>> {
    match (uri.path(), uri.query()) {
        ("*", None) => unimplemented!("Should return asterisk resource"),
        (path, query) if path.starts_with('/') => {
            let queryable_resource = lookup
                .lookup(&path[1..])
                .await
                .map_err(ResolveError::GoodError)?;
            queryable_resource
                .query(query)
                .map_err(ResolveError::LookupError)
        }
        _ => Err(ResolveError::MalformedUri(uri)),
    }
}

fn bad_request() -> resource::Response {
    resource::Response::new(
        Status::BadRequest,
        vec![(
            MediaType::new("text", "plain", vec![]),
            Box::new(move || Box::new("Bad Request\n") as RepresentationBox) as _,
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
) -> Result<
    (
        Option<ETag>,
        resource::Response,
        Option<resource::CacheControl>,
    ),
    Error,
> {
    let (req, body) = req.into_parts();

    let cookie_handler: Box<dyn CookieHandler + Send> = resolve_resource(site, &req.uri)
        .await
        .map_err(|x| match x {
            ResolveError::MalformedUri(_) => Error::BadRequest,
            ResolveError::LookupError(_) => Error::InternalServerError,
            ResolveError::GoodError(x) => Error::BlanketResponse(x),
        })?;

    let read_cookies = cookie_handler.read_cookies();

    let cookies = if read_cookies.len() > 0 {
        // TODO Set Vary: Cookie
        // Propagate this via return value? Change to struct?

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

        cookies
    } else {
        vec![] // Allocation should be unnecessary
    };

    let resource = cookie_handler.cookies(&cookies).await?;

    let etag = resource.etag.clone();

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

    match req.method {
        // TODO: Implement HEAD and OPTIONS in library
        hyper::Method::GET => {
            let (response, cache_control) = resource.get().await;
            return Ok((etag, response, cache_control));
        }
        hyper::Method::POST => {
            let content_type = req
                .headers
                .get(http::header::CONTENT_TYPE)
                .map(|x| x.to_str().map(|x| x.to_string())); // TODO should be parsed as a MediaType

            if let Some(Ok(content_type)) = content_type {
                let response = resource.post(content_type, body).await;
                return Ok((etag, response, None));
            } else {
                return Ok((etag, bad_request(), None));
            }
        }
        _ => return Ok((etag, resource.method_not_allowed(), None)),
    };
}

use hyper::http::StatusCode;

async fn build_response(
    etag: Option<ETag>,
    response: resource::Response,
    cache_control: Option<resource::CacheControl>,
) -> hyper::Response<Body> {
    let resource::Response {
        status,
        mut representations,
        cookies,
    } = response;

    let mut response = hyper::Response::builder();

    match status {
        // 2__
        Status::Ok => {
            response.status(StatusCode::OK);
        }
        Status::Created(location) => {
            response.status(StatusCode::CREATED);
            response.header("location", location);
        }

        // 3__
        Status::MovedPermanently(location) => {
            response.status(StatusCode::MOVED_PERMANENTLY);
            response.header("location", location);
        }

        Status::SeeOther(location) => {
            response.status(StatusCode::SEE_OTHER);
            response.header("location", location);
        }

        // 4__
        Status::BadRequest => {
            response.status(StatusCode::BAD_REQUEST);
        }

        Status::Unauthorized => {
            response.status(StatusCode::UNAUTHORIZED);
            // TODO: Set `WWW-Authenticate` header
        }

        Status::MethodNotAllowed { allow } => {
            response.status(StatusCode::METHOD_NOT_ALLOWED);
            response.header("allow", allow);
        }

        Status::NotFound => {
            response.status(StatusCode::NOT_FOUND);
        }

        // 5__
        Status::InternalServerError => {
            response.status(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

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

    if let Some(cache_control) = cache_control {
        let mut cc = String::new();

        if cache_control.cacheability.private {
            write!(&mut cc, "private").unwrap();
        } else {
            write!(&mut cc, "public").unwrap();
        }

        match cache_control.cacheability.policy {
            resource::CacheabilityPolicy::AllowCaching => (),
            resource::CacheabilityPolicy::NoCache => write!(&mut cc, ", no-cache").unwrap(),
            resource::CacheabilityPolicy::NoStore => write!(&mut cc, ", no-store").unwrap(),
        };

        if cache_control.revalidation.must_revalidate {
            write!(&mut cc, ", must-revalidate").unwrap();
        }

        if cache_control.revalidation.proxy_revalidate {
            write!(&mut cc, ", proxy-revalidate").unwrap();
        }

        if cache_control.revalidation.immutable {
            write!(&mut cc, ", max-age=31536000, immutable").unwrap();
        }

        response.header("cache-control", cc);
    }

    if cookies.len() > 0 {
        response.header(
            "set-cookie",
            cookies
                .into_iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        );
    }

    response
        .body(representation.body())
        .expect("Success should be guaranteed at type level")
}

async fn handle_request_core<'a>(
    site: &'a (dyn Lookup + 'a + Send + Sync),
    req: Request<Body>,
) -> hyper::Response<Body> {
    let (etag, response, cache_control) =
        try_handle_request(site, req)
            .await
            .unwrap_or_else(|err| match err {
                Error::BadRequest => unimplemented!(),
                Error::InternalServerError => unimplemented!(),
                Error::BlanketResponse(r) => (None, r, None),
            });

    build_response(etag, response, cache_control).await
}

// This exists merely to allow use of .compat() layer for futures 0.1 support
pub async fn handle_request<'a, L>(
    site: std::sync::Arc<L>,
    req: Request<Body>,
) -> Result<hyper::Response<Body>, Box<dyn std::error::Error + Send + Sync + 'static>>
where
    L: Lookup + 'a + Send + Sync,
{
    Ok(handle_request_core(&*site, req).await)
}
