use futures::{compat::Stream01CompatExt, FutureExt, TryStreamExt};
use hyper::http;
use serde_urlencoded;
use web::{Error, FutureBox, MediaType, RepresentationBox, RepresentationsVec, Resource};

use super::handling_error::HandlingError;
use super::auth;

pub struct Index {
    claims: Option<auth::Claims>
}

#[derive(BartDisplay)]
#[template = "templates/index.html"]
struct Get<'a> {
    claims: &'a Option<auth::Claims>
}

#[derive(serde_derive::Deserialize)]
struct PostArgs {
    email: String,
}

#[derive(BartDisplay)]
#[template = "templates/index-post.html"]
struct Post<'a> {
    email: &'a str,
}

impl Index {
    async fn try_post(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> Result<(http::StatusCode, RepresentationsVec), HandlingError> {
        let content_type = content_type;
        if content_type != "application/x-www-form-urlencoded" {
            return Err(HandlingError::BadRequest(
                "Unacceptable Content-Type, must be application/x-www-form-urlencoded",
            ));
        }

        let body = body
            .compat()
            .try_concat()
            .await
            .map_err(|_| HandlingError::InternalServerError)?;
        let args: PostArgs = serde_urlencoded::from_bytes(&body)
            .map_err(|_| HandlingError::BadRequest("Invalid data"))?; // TODO Use given error.to_string()

        Ok((
            http::StatusCode::OK,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(Post { email: &args.email }.to_string()) as RepresentationBox
                }) as _,
            )],
        ))
    }

    async fn post_core(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> (http::StatusCode, RepresentationsVec) {
        self.try_post(content_type, body)
            .await
            .unwrap_or_else(|e| e.render())
    }
}

impl Resource for Index {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        async {
            (
                http::StatusCode::OK,
                vec![(
                    MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                    Box::new(move || Box::new(Get { claims: &self.claims }.to_string()) as RepresentationBox) as _,
                )],
            )
        }
            .boxed()
    }

    fn post<'a>(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        self.post_core(content_type, body).boxed()
    }
}

pub struct IndexLoader;

impl auth::ClaimsConsumer for IndexLoader {
    type Claims = auth::Claims;

    fn claims<'a>(
        self,
        claims: Option<Self::Claims>,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>> {
        async { Ok(Box::new(Index { claims }) as Box<dyn Resource + Send + 'static>) }.boxed() as _
    }
}
