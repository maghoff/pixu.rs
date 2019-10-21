use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::FutureExt;
use hyper::http;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{Error, FutureBox, MediaType, RepresentationBox, Resource, Response};

use super::auth;
use super::handling_error::HandlingError;
use crate::db::schema::*;

pub struct Index {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    claims: Option<auth::Claims>,
}

#[derive(BartDisplay)]
#[template = "templates/index.html"]
struct Get<'a> {
    claims: &'a Option<auth::Claims>,
    authorized_pixurs: &'a [i32],
}

impl Index {
    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        let authorized_pixurs = self
            .claims
            .as_ref()
            .map(|claims| {
                pixur_authorizations::table
                    .filter(pixur_authorizations::sub.eq(&claims.sub))
                    .select(pixur_authorizations::pixur_id)
                    .load::<i32>(&*db_connection)
            })
            .transpose()
            .map_err(|_| HandlingError::InternalServerError)?
            .unwrap_or_else(|| vec![]);

        Ok(Response::new(
            http::StatusCode::OK,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(
                        Get {
                            claims: &self.claims,
                            authorized_pixurs: &authorized_pixurs,
                        }
                        .to_string(),
                    ) as RepresentationBox
                }) as _,
            )],
        ))
    }

    async fn get_core(self: Box<Self>) -> Response {
        self.try_get().await.unwrap_or_else(|e| e.render())
    }
}

impl Resource for Index {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        self.get_core().boxed()
    }
}

pub struct IndexLoader {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl auth::ClaimsConsumer for IndexLoader {
    type Claims = auth::Claims;

    fn claims<'a>(
        self,
        claims: Option<Self::Claims>,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>> {
        async {
            Ok(Box::new(Index {
                claims,
                db_pool: self.db_pool,
            }) as Box<dyn Resource + Send + 'static>)
        }
        .boxed() as _
    }
}
