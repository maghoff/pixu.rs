use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use hyper::http;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{MediaType, RepresentationBox, RepresentationsVec, Resource};

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

    fn try_get(self: Box<Self>) -> Result<(http::StatusCode, RepresentationsVec), HandlingError> {
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

    fn get_core(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        self.try_get().unwrap_or_else(|e| e.render())
    }
}

impl Resource for Image {
    fn get(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        // TODO Async GET handling

        self.get_core()
    }
}
