use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{Get, MediaType, RepresentationBox, Resource, Response};

use super::auth;
use super::handling_error::HandlingError;
use crate::db::schema::*;
use crate::id30::Id30;

pub struct Thumbnail {
    title: String,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    id: Id30,
}

impl Thumbnail {
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
                Box::new(move || Box::new(pix.data) as RepresentationBox),
            )],
        ))
    }
}

#[async_trait::async_trait]
impl Get for Thumbnail {
    fn cache_control(&self) -> web::CacheControl {
        web::CacheControl {
            cacheability: web::Cacheability {
                private: true,
                policy: web::CacheabilityPolicy::AllowCaching,
            },
            revalidation: web::Revalidation {
                must_revalidate: false,
                proxy_revalidate: false,
                immutable: true,
            },
        }
    }

    async fn representations(self: Box<Self>) -> Response {
        let title = self.title.clone();

        self.try_get().await.unwrap_or_else(|e| e.render(&title))
    }
}
pub struct AuthorizationConsumer {
    pub title: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl auth::authorizer::Consumer for AuthorizationConsumer {
    type Authorization = Id30;

    fn authorization<'a>(self, id: Id30) -> Result<Resource, web::Error> {
        Ok(Resource {
            etag: None,
            get: Some(Box::new(Thumbnail {
                title: self.title,
                db_pool: self.db_pool,
                id,
            })),
            post: None,
        })
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

        let is_uploader = select(exists(uploaders::table.filter(uploaders::sub.eq(sub))))
            .first(&*db_connection)
            .expect("Query must return 1 result");

        let authorized = is_uploader
            || select(exists(
                pixur_authorizations::table
                    .inner_join(pixurs::table)
                    .filter(pixurs::thumbs_id.eq(self.id))
                    .filter(pixur_authorizations::sub.eq(sub)),
            ))
            .first(&*db_connection)
            .expect("Query must return 1 result");

        if authorized {
            Ok(Some(self.id))
        } else {
            Ok(None)
        }
    }
}
