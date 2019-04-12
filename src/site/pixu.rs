use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use hyper::http;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{MediaType, RepresentationBox, RepresentationsVec, Resource};

use super::handling_error::HandlingError;
use crate::db::schema::*;

pub struct Pixu {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    id: i32,
}

#[derive(BartDisplay)]
#[template = "templates/pixu.html"]
struct Get<'a> {
    average_color: &'a str,
    thumb_url: &'a str,
    large_url: &'a str,
}

impl Pixu {
    pub fn new(db_pool: Pool<ConnectionManager<SqliteConnection>>, id: i32) -> Pixu {
        Pixu { db_pool, id }
    }

    fn try_get(self: Box<Self>) -> Result<(http::StatusCode, RepresentationsVec), HandlingError> {
        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        #[derive(Queryable)]
        struct Pixurs {
            #[allow(unused)]
            id: i32,
            average_color: i32,
            thumbs_id: i32,
        }

        let pix: Pixurs = pixurs::table
            .filter(pixurs::id.eq(self.id))
            .first(&*db_connection)
            .map_err(|_| HandlingError::InternalServerError)?;

        let large_id: i32 = images_meta::table
            .filter(images_meta::pixurs_id.eq(self.id))
            .order(images_meta::width.desc())
            .select(images_meta::id)
            .first(&*db_connection)
            .map_err(|_| HandlingError::InternalServerError)?;

        Ok((
            http::StatusCode::OK,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(
                        Get {
                            average_color: &format!("#{:06x}", pix.average_color),
                            thumb_url: &format!("thumb/{}", pix.thumbs_id),
                            large_url: &format!("img/{}", large_id),
                        }
                        .to_string(),
                    ) as RepresentationBox
                }) as _,
            )],
        ))
    }

    fn get_core(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        self.try_get().unwrap_or_else(|e| e.render())
    }
}

impl Resource for Pixu {
    fn get(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        // TODO Async GET handling

        self.get_core()
    }
}
