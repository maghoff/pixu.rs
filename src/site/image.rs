use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::future::FutureExt;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{FutureBox, MediaType, RepresentationBox, Resource, Response};

use super::auth;
use super::handling_error::HandlingError;
use crate::db::schema::*;
use crate::id30::Id30;

pub struct Image {
    title: String,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    id: Id30,
}

impl Image {
    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        #[derive(Queryable)]
        struct Image {
            #[allow(unused)]
            id: i32,
            media_type: String,
            data: Vec<u8>,
        }

        // TODO Schedule IO operation on some kind of background thread
        // Maybe using spawn_blocking()?
        let pix: Image = images::table
            .filter(images::id.eq(self.id))
            .first(&*db_connection)
            .map_err(|_| HandlingError::InternalServerError)?;

        Ok(Response::new(
            web::Status::Ok,
            vec![(
                MediaType::parse(&pix.media_type),
                Box::new(move || Box::new(pix.data) as RepresentationBox) as _,
            )],
        ))
    }

    async fn get_core(self: Box<Self>) -> Response {
        let title = self.title.clone();

        self.try_get().await.unwrap_or_else(|e| e.render(&title))
    }
}

impl Resource for Image {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        self.get_core().boxed()
    }
}

pub struct AuthorizationConsumer {
    pub title: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl auth::authorizer::Consumer for AuthorizationConsumer {
    type Authorization = Id30;

    fn authorization<'a>(self, id: Id30) -> Result<Box<dyn Resource + Send + 'static>, web::Error> {
        Ok(Box::new(Image {
            title: self.title,
            db_pool: self.db_pool,
            id,
        }) as _)
    }
}

pub struct AuthorizationProvider {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub id: Id30,
}

impl auth::authorizer::Provider for AuthorizationProvider {
    type Authorization = Id30;

    fn get_authorization(&self, sub: &str) -> Result<Option<Self::Authorization>, web::Error> {
        use diesel::dsl::*;

        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| web::Error::InternalServerError)?;

        let authorized: bool = select(exists(
            pixur_authorizations::table
                .inner_join(pixurs::table.inner_join(images_meta::table))
                .filter(images_meta::id.eq(self.id))
                .filter(pixur_authorizations::sub.eq(sub)),
        ))
        .first::<bool>(&*db_connection)
        .expect("Query must return 1 result");

        if authorized {
            Ok(Some(self.id))
        } else {
            Ok(None)
        }
    }
}
