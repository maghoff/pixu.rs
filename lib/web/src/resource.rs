use core::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use cookie::Cookie;

use super::etag::ETag;
use super::media_type::MediaType;
use super::representation::Representation;

pub type RepresentationBox = Box<dyn Representation + Send + 'static>;
pub type RendererBox = Box<dyn FnOnce() -> RepresentationBox + Send + 'static>;
pub type RepresentationsVec = Vec<(MediaType, RendererBox)>;
pub type FutureBox<'a, Output> = Pin<Box<dyn Future<Output = Output> + Send + 'a>>;

#[derive(PartialEq, Eq, Debug)]
pub enum Status {
    // 2__
    Ok,
    Created(String),

    // 3__
    MovedPermanently(String),
    SeeOther(String),

    // 4__
    BadRequest,
    Unauthorized, // TODO: `WWW-Authenticate` header
    NotFound,
    MethodNotAllowed { allow: String },

    // 5__
    InternalServerError,
}

pub enum CacheabilityPolicy {
    AllowCaching,
    NoCache, //< User-agent must revalidate before using cached response
    NoStore,
}

impl Default for CacheabilityPolicy {
    fn default() -> Self {
        CacheabilityPolicy::AllowCaching
    }
}

pub struct Cacheability {
    pub private: bool,
    pub policy: CacheabilityPolicy,
}

impl Default for Cacheability {
    fn default() -> Self {
        Cacheability {
            private: true,
            policy: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct Revalidation {
    pub must_revalidate: bool,
    pub proxy_revalidate: bool,
    pub immutable: bool,
}

#[derive(Default)]
pub struct CacheControl {
    pub cacheability: Cacheability,
    // expiration, see https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control#Expiration
    pub revalidation: Revalidation,
    // other, see https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Cache-Control#Other
}

impl std::fmt::Display for CacheControl {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.cacheability.private {
            write!(fmt, "private")?;
        } else {
            write!(fmt, "public")?;
        }

        match self.cacheability.policy {
            CacheabilityPolicy::AllowCaching => (),
            CacheabilityPolicy::NoCache => write!(fmt, ", no-cache")?,
            CacheabilityPolicy::NoStore => write!(fmt, ", no-store")?,
        };

        if self.revalidation.must_revalidate {
            write!(fmt, ", must-revalidate")?;
        }

        if self.revalidation.proxy_revalidate {
            write!(fmt, ", proxy-revalidate")?;
        }

        if self.revalidation.immutable {
            write!(fmt, ", max-age=31536000, immutable")?;
        }

        Ok(())
    }
}

pub struct Response {
    pub status: Status,
    pub representations: RepresentationsVec,
    pub cookies: Vec<Cookie<'static>>,
}

impl Response {
    pub fn new(status: Status, representations: RepresentationsVec) -> Response {
        Response {
            status,
            representations,
            cookies: vec![],
        }
    }
}

#[async_trait]
pub trait Get {
    // TODO Would it be better to have this unconditionally return CacheControl?
    fn cache_control(&self) -> Option<CacheControl> {
        None
    }

    async fn representations(self: Box<Self>) -> Response;
}

#[async_trait]
pub trait Post {
    async fn post(self: Box<Self>, content_type: String, body: hyper::Body) -> Response;
}

pub struct Resource {
    pub etag: Option<ETag>,
    pub get: Option<Box<dyn Get + Send>>,
    pub post: Option<Box<dyn Post + Send>>,
}

impl Resource {
    pub fn method_not_allowed(&self) -> Response {
        let mut allow = "OPTIONS".to_string();
        if self.get.is_some() {
            allow.push_str(", GET, HEAD");
        }
        if self.post.is_some() {
            allow.push_str(", POST");
        }

        Response::new(
            Status::MethodNotAllowed { allow },
            vec![(
                MediaType::new("text", "plain", vec![]),
                Box::new(move || Box::new("Method Not Allowed\n") as RepresentationBox) as _,
            )],
        )
    }

    pub async fn get(self) -> (Response, Option<CacheControl>) {
        match self.get {
            Some(get) => {
                let cache_control = get.cache_control();
                (get.representations().await, cache_control)
            }
            None => (self.method_not_allowed(), None),
        }
    }

    pub async fn post(self, content_type: String, body: hyper::Body) -> Response {
        match self.post {
            Some(post) => post.post(content_type, body).await,
            None => self.method_not_allowed(),
        }
    }
}
