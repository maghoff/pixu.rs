use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{Error, MediaType, RepresentationBox, Resource, Response};

use super::auth;
use super::handling_error::HandlingError;
use crate::db::schema::*;
use crate::id30::Id30;

pub struct Index {
    title: String,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    self_url: String,
    claims: Option<auth::Claims>,
}

struct UploaderExtra<'a> {
    recipients: Vec<String>,
    url: &'a str,
    message: &'a str,
}

#[derive(BartDisplay)]
#[template = "templates/index.html"]
struct Get<'a> {
    self_url: &'a str,
    claims: &'a Option<auth::Claims>,
    is_uploader: Option<UploaderExtra<'a>>,
    authorized_pixurs: &'a [(Id30, Id30, Id30)],
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
                let recipients = pixur_series_authorizations::table
                    .select(pixur_series_authorizations::sub)
                    .order(pixur_series_authorizations::sub.asc())
                    .distinct()
                    .load::<String>(&*db_connection)
                    .map_err(|_| HandlingError::InternalServerError)?;

                Some(UploaderExtra {
                    recipients,
                    message: "",
                    url: "",
                })
            }
        };

        let authorized_pixurs = self
            .claims
            .as_ref()
            .map(|claims| {
                if is_uploader.is_some() {
                    pixurs::table
                        .inner_join(images_meta::table)
                        .order(pixurs::created.desc())
                        .select((pixurs::id, pixurs::thumbs_id, images_meta::id))
                        .load::<(Id30, Id30, Id30)>(&*db_connection)
                } else {
                    pixur_series_authorizations::table
                        .inner_join(
                            pixur_series::table
                                .inner_join(pixurs::table.inner_join(images_meta::table))
                                .on(pixur_series::id
                                    .eq(pixur_series_authorizations::pixur_series_id)),
                        )
                        .order(pixurs::created.desc())
                        .filter(pixur_series_authorizations::sub.eq(&claims.sub))
                        .select((
                            pixur_series_authorizations::pixur_series_id,
                            pixurs::thumbs_id,
                            images_meta::id,
                        ))
                        .load::<(Id30, Id30, Id30)>(&*db_connection)
                }
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
                            title: &self.title,
                            body: &Get {
                                self_url: &self.self_url,
                                claims: &self.claims,
                                is_uploader,
                                authorized_pixurs: &authorized_pixurs,
                            },
                        }
                        .to_string(),
                    ) as RepresentationBox
                }),
            )],
        ))
    }
}

#[async_trait::async_trait]
impl web::Get for Index {
    async fn representations(self: Box<Self>) -> Response {
        let title = self.title.clone();

        self.try_get().await.unwrap_or_else(|e| e.render(&title))
    }
}

pub struct IndexLoader {
    pub title: String,
    pub self_url: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

#[async_trait::async_trait]
impl auth::ClaimsConsumer for IndexLoader {
    type Claims = auth::Claims;

    async fn claims(self, claims: Option<Self::Claims>) -> Result<Resource, Error> {
        Ok(Resource {
            etag: None,
            get: Some(Box::new(Index {
                title: self.title,
                claims,
                self_url: self.self_url,
                db_pool: self.db_pool,
            })),
            post: None,
        })
    }
}
