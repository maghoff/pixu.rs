use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::future::FutureExt;
use hyper::http;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{Error, FutureBox, MediaType, RepresentationBox, RepresentationsVec, Resource};

use super::auth;
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

    async fn try_get(
        self: Box<Self>,
    ) -> Result<(http::StatusCode, RepresentationsVec), HandlingError> {
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

        // TODO Schedule IO operations on some kind of background thread

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

    async fn get_core(self: Box<Self>) -> (http::StatusCode, RepresentationsVec) {
        self.try_get().await.unwrap_or_else(|e| e.render())
    }

    async fn claims_core<'a>(
        self,
        claims: auth::Claims,
    ) -> Result<Box<dyn Resource + Send + 'static>, Error> {
        let db_connection = self.db_pool.get().map_err(|_| Error::InternalServerError)?;

        let authorized: bool = pixur_authorizations::table
            .filter(pixur_authorizations::pixur_id.eq(self.id))
            .filter(pixur_authorizations::sub.eq(claims.sub))
            .count()
            .first::<i64>(&*db_connection)
            .map_err(|_| Error::InternalServerError)?
            != 0;

        if authorized {
            Ok(Box::new(self) as Box<dyn Resource + Send + 'static>)
        } else {
            unimplemented!()
        }
    }
}

impl Resource for Pixu {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, (http::StatusCode, RepresentationsVec)> {
        self.get_core().boxed()
    }
}

impl auth::ClaimsConsumer for Pixu {
    type Claims = auth::Claims;

    fn claims<'a>(
        self,
        claims: Self::Claims,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>> {
        self.claims_core(claims).boxed()
    }
}
