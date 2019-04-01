use futures::{compat::Stream01CompatExt, FutureExt, TryStreamExt};
use hyper::http;
use serde_urlencoded;
use web::{FutureBox, MediaType, RepresentationBox, RepresentationsVec, Resource};

use super::handling_error::HandlingError;

pub struct Index;

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
        await!(self.try_post(content_type, body)).unwrap_or_else(|e| e.render())
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
    ) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        self.post_core(content_type, body).boxed()
    }
}
