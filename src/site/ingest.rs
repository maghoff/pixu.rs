use diesel;
use diesel::sqlite::SqliteConnection;
use futures::{compat::Stream01CompatExt, TryStreamExt};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{Post, Resource, Response};

use super::auth;
use super::auth_provider;
use super::handling_error::HandlingError;
use crate::image;

pub struct Ingest {
    pub title: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub base_url: String,
}

impl Ingest {
    async fn try_post(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> Result<Response, HandlingError> {
        // TODO Real parsing of media type syntax
        if !content_type.starts_with("image/jpeg") {
            return Err(HandlingError::BadRequest(
                "Unacceptable Content-Type, must be image/jpeg",
            ));
        }

        let body = body
            .compat()
            .try_concat()
            .await
            .map_err(|_| HandlingError::InternalServerError)?;

        let (id, series_id) = image::ingest_jpeg(&body, self.db_pool)
            .map_err(|_| HandlingError::InternalServerError)?;

        let url = format!("{}{}", self.base_url, id);
        let series_url = format!("{}{}", self.base_url, series_id);

        #[derive(serde_derive::Serialize)]
        struct IngestResponse<'a> {
            url: &'a str,
            series_url: &'a str,
        }

        let json = serde_json::to_string(&IngestResponse {
            url: &url,
            series_url: &series_url,
        })
        .map_err(|_| HandlingError::InternalServerError)?;

        Ok(Response {
            status: web::Status::Created(url),
            representations: vec![(
                web::MediaType::new("application", "json", vec![]),
                Box::new(move || Box::new(json) as web::RepresentationBox),
            )],
            cookies: vec![],
        })
    }
}

#[async_trait::async_trait]
impl Post for Ingest {
    async fn post(self: Box<Self>, content_type: String, body: hyper::Body) -> Response {
        let title = self.title.clone();

        self.try_post(content_type, body)
            .await
            .unwrap_or_else(|e| e.render(&title))
    }
}

pub struct AuthorizationConsumer {
    pub title: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub base_url: String,
}

impl auth::authorizer::Consumer for AuthorizationConsumer {
    type Authorization = auth_provider::CanEdit;

    fn authorization(self, _: Self::Authorization) -> Result<Resource, web::Error> {
        Ok(Resource {
            etag: None,
            get: None,
            post: Some(Box::new(Ingest {
                title: self.title,
                db_pool: self.db_pool,
                base_url: self.base_url,
            })),
        })
    }
}
