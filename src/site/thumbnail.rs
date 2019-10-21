use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::future::FutureExt;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{FutureBox, MediaType, RepresentationBox, Resource, Response};

use super::handling_error::HandlingError;
use crate::db::schema::*;

pub struct Thumbnail {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    id: i32,
}

impl Thumbnail {
    pub fn new(db_pool: Pool<ConnectionManager<SqliteConnection>>, id: i32) -> Thumbnail {
        Thumbnail { db_pool, id }
    }

    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        #[derive(Queryable)]
        struct Thumbnail {
            #[allow(unused)]
            id: i32,
            media_type: String,
            data: Vec<u8>,
        }

        // TODO Schedule IO operation on some kind of background thread
        let pix: Thumbnail = thumbs::table
            .filter(thumbs::id.eq(self.id))
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
        self.try_get().await.unwrap_or_else(|e| e.render())
    }
}

impl Resource for Thumbnail {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        self.get_core().boxed()
    }
}
