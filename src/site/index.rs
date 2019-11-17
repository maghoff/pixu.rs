use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::FutureExt;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{Error, FutureBox, MediaType, RepresentationBox, Resource, Response};

use super::auth;
use super::handling_error::HandlingError;
use crate::db::schema::*;
use crate::id30::Id30;

pub struct Index {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    self_url: String,
    claims: Option<auth::Claims>,
}

struct UploaderExtra {
    recipients: Vec<String>,
}

#[derive(BartDisplay)]
#[template = "templates/index.html"]
struct Get<'a> {
    self_url: &'a str,
    claims: &'a Option<auth::Claims>,
    is_uploader: Option<UploaderExtra>,
    authorized_pixurs: &'a [(Id30, Id30)],
}

impl Index {
    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        use diesel::dsl::*;

        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        let is_uploader = self
            .claims
            .as_ref()
            .map(|claims| {
                select(exists(
                    uploaders::table.filter(uploaders::sub.eq(&claims.sub)),
                ))
                .first::<bool>(&*db_connection)
            })
            .transpose()
            .map_err(|_| HandlingError::InternalServerError)?
            .unwrap_or(false);

        let is_uploader = match is_uploader {
            false => None,
            true => {
                let recipients = pixur_authorizations::table
                    .select(pixur_authorizations::sub)
                    .order(pixur_authorizations::sub.asc())
                    .distinct()
                    .load::<String>(&*db_connection)
                    .map_err(|_| HandlingError::InternalServerError)?;

                Some(UploaderExtra { recipients })
            }
        };

        let authorized_pixurs = self
            .claims
            .as_ref()
            .map(|claims| {
                pixur_authorizations::table
                    .inner_join(pixurs::table)
                    .filter(pixur_authorizations::sub.eq(&claims.sub))
                    .select((pixur_authorizations::pixur_id, pixurs::thumbs_id))
                    .load::<(Id30, Id30)>(&*db_connection)
            })
            .transpose()
            .map_err(|_| HandlingError::InternalServerError)?
            .unwrap_or_else(|| vec![]);

        Ok(Response::new(
            web::Status::Ok,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(
                        super::Layout {
                            body: &Get {
                                self_url: &self.self_url,
                                claims: &self.claims,
                                is_uploader,
                                authorized_pixurs: &authorized_pixurs,
                            },
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
    pub self_url: String,
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
                self_url: self.self_url,
                db_pool: self.db_pool,
            }) as Box<dyn Resource + Send + 'static>)
        }
        .boxed() as _
    }
}
