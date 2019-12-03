use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::{compat::Stream01CompatExt, FutureExt, TryStreamExt};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{FutureBox, Resource, Response};

use super::auth;
use super::handling_error::HandlingError;
use crate::db::schema::*;
use crate::image;

pub struct Ingest {
    pub title: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
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

        let id = image::ingest_jpeg(&body, self.db_pool)
            .map_err(|_| HandlingError::InternalServerError)?;

        Ok(Response {
            status: web::Status::Created(id.to_string()), // TODO Use base_url
            representations: vec![(
                web::MediaType::new("image", "jpeg", vec![]),
                Box::new(move || Box::new(body) as web::RepresentationBox) as _,
            )],
            cookies: vec![],
        })
    }

    async fn async_post(self: Box<Self>, content_type: String, body: hyper::Body) -> Response {
        let title = self.title.clone();

        self.try_post(content_type, body)
            .await
            .unwrap_or_else(|e| e.render(&title))
    }
}

impl Resource for Ingest {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        unimplemented!()
    }

    fn post<'a>(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> FutureBox<'a, Response> {
        self.async_post(content_type, body).boxed()
    }
}

pub struct AuthorizationConsumer {
    pub title: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl auth::authorizer::Consumer for AuthorizationConsumer {
    type Authorization = ();

    fn authorization<'a>(self, _: ()) -> Result<Box<dyn Resource + Send + 'static>, web::Error> {
        Ok(Box::new(Ingest {
            title: self.title,
            db_pool: self.db_pool,
        }) as _)
    }
}

pub struct AuthorizationProvider {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl auth::authorizer::Provider for AuthorizationProvider {
    type Authorization = ();

    fn get_authorization(&self, sub: &str) -> Result<Option<Self::Authorization>, web::Error> {
        use diesel::dsl::*;

        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| web::Error::InternalServerError)?;

        let authorized: bool = select(exists(uploaders::table.filter(uploaders::sub.eq(sub))))
            .first::<bool>(&*db_connection)
            .expect("Query must return 1 result");

        if authorized {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}
