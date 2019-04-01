use core::future::Future;
use std::pin::Pin;

use futures::{compat::Stream01CompatExt, FutureExt, TryStreamExt};
use hyper::http;
use serde_urlencoded;
use web::{Lookup, MediaType, QueryableResource, Representation, Resource};

type RepresentationBox = Box<dyn Representation + Send + 'static>;
type RendererBox = Box<dyn FnOnce() -> RepresentationBox + Send + 'static>;
type RepresentationsVec = Vec<(MediaType, RendererBox)>;

enum HandlingError {
    BadRequest(&'static str),
    InternalServerError,
}

struct Index;

impl Index {
    async fn try_post(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> Result<(http::StatusCode, RepresentationsVec), HandlingError> {
        #[derive(serde_derive::Deserialize)]
        struct Args {
            email: String,
        }

        #[derive(BartDisplay)]
        #[template = "templates/index-post.html"]
        struct Template<'a> {
            email: &'a str,
        }

        let content_type = content_type;
        if content_type != "application/x-www-form-urlencoded" {
            return Err(HandlingError::BadRequest(
                "Unacceptable Content-Type, must be application/x-www-form-urlencoded",
            ));
        }

        let body = await! { body.compat().try_concat() }
            .map_err(|_| HandlingError::InternalServerError)?;
        let args: Args = serde_urlencoded::from_bytes(&body)
            .map_err(|_| HandlingError::BadRequest("Invalid data"))?;

        Ok((
            http::StatusCode::OK,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(Template { email: &args.email }.to_string()) as RepresentationBox
                }) as _,
            )],
        ))
    }

    async fn post_core(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> (http::StatusCode, RepresentationsVec) {
        #[derive(BartDisplay)]
        #[template = "templates/err/bad-request.html"]
        struct BadRequest<'a> {
            details: &'a str,
        }

        #[derive(BartDisplay)]
        #[template = "templates/err/internal-server-error.html"]
        struct InternalServerError;

        match await! { self.try_post(content_type, body) } {
            Ok(x) => x,
            Err(HandlingError::BadRequest(details)) => (
                http::StatusCode::BAD_REQUEST,
                vec![(
                    MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                    Box::new(move || {
                        Box::new(BadRequest { details }.to_string()) as RepresentationBox
                    }) as _,
                )],
            ),
            Err(HandlingError::InternalServerError) => (
                http::StatusCode::BAD_REQUEST,
                vec![(
                    MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                    Box::new(move || Box::new(InternalServerError.to_string()) as RepresentationBox)
                        as _,
                )],
            ),
        }
    }
}

impl Resource for Index {
    fn get(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        #[derive(BartDisplay)]
        #[template = "templates/index.html"]
        struct Template;

        (
            http::StatusCode::OK,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || Box::new(Template.to_string()) as RepresentationBox) as _,
            )],
        )
    }

    fn post<'a>(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> Pin<Box<dyn Future<Output = (http::StatusCode, RepresentationsVec)> + Send + 'a>> {
        self.post_core(content_type, body).boxed()
    }
}

fn not_found() -> impl QueryableResource {
    #[derive(BartDisplay)]
    #[template_string = "Not found!\n"]
    struct NotFound;

    (
        http::StatusCode::NOT_FOUND,
        vec![(
            MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || Box::new(NotFound.to_string()) as RepresentationBox) as _,
        )],
    )
}

pub async fn lookup(path: &str) -> Box<dyn QueryableResource> {
    match path {
        "" => Box::new(Index) as _,
        _ => Box::new(not_found()) as _,
    }
}

pub struct Site;

impl Lookup for Site {
    fn lookup<'a>(
        &'a self,
        path: &'a str,
    ) -> Pin<Box<dyn Future<Output = Box<dyn QueryableResource>> + Send + 'a>> {
        lookup(&path).boxed()
    }
}
