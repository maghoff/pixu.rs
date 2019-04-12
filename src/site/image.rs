use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::future::FutureExt;
use hyper::http;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{FutureBox, MediaType, RepresentationBox, RepresentationsVec, Resource};

use super::handling_error::HandlingError;
use crate::db::schema::*;

pub struct Image {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    id: i32,
}

impl Image {
    pub fn new(db_pool: Pool<ConnectionManager<SqliteConnection>>, id: i32) -> Image {
        Image { db_pool, id }
    }

    async fn try_get(
        self: Box<Self>,
    ) -> Result<(http::StatusCode, RepresentationsVec), HandlingError> {
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
        let pix: Image = images::table
            .filter(images::id.eq(self.id))
            .first(&*db_connection)
            .map_err(|_| HandlingError::InternalServerError)?;

        Ok((
            http::StatusCode::OK,
            vec![(
                MediaType::parse(&pix.media_type),
                Box::new(move || Box::new(pix.data) as RepresentationBox) as _,
            )],
        ))
    }

    async fn get_core(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        await!(self.try_get()).unwrap_or_else(|e| e.render())
    }
}

impl Resource for Image {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        self.get_core().boxed()
    }
}
