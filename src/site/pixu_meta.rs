use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::{compat::Stream01CompatExt, FutureExt, TryStreamExt};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{Error, FutureBox, MediaType, RepresentationBox, Resource, Response};

use super::auth;
use super::handling_error::HandlingError;
use crate::db::schema::*;
use crate::id30::Id30;

pub struct PixuMeta {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    id: Id30,
}

#[derive(serde_derive::Serialize)]
struct MetadataGet {
    recipients: Vec<String>,
}

#[derive(serde_derive::Deserialize)]
struct MetadataPost<'a> {
    #[serde(borrow)]
    recipients: std::collections::BTreeSet<&'a str>,
}

#[derive(serde_derive::Deserialize)]
struct UpdateRequest<'a> {
    #[serde(borrow)]
    metadata: MetadataPost<'a>,
}

impl PixuMeta {
    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        let recipients = pixur_authorizations::table
            .filter(pixur_authorizations::pixur_id.eq(self.id))
            .select(pixur_authorizations::sub)
            .load(&*db_connection)
            .map_err(|_| HandlingError::InternalServerError)?;

        let metadata = MetadataGet { recipients };

        let json =
            serde_json::to_string(&metadata).map_err(|_| HandlingError::InternalServerError)?;

        Ok(Response::new(
            web::Status::Ok,
            vec![(
                MediaType::new("application", "json", vec![]),
                Box::new(move || Box::new(json) as RepresentationBox) as _,
            )],
        ))
    }

    async fn async_get(self: Box<Self>) -> Response {
        self.try_get().await.unwrap_or_else(|e| e.render())
    }

    async fn try_post(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> Result<Response, HandlingError> {
        let content_type = content_type;
        if content_type != "application/json" {
            return Err(HandlingError::BadRequest(
                "Unacceptable Content-Type, must be application/json",
            ));
        }

        let body = body
            .compat()
            .try_concat()
            .await
            .map_err(|_| HandlingError::InternalServerError)?;

        let update_request: UpdateRequest =
            serde_json::from_slice(&body).map_err(|_| HandlingError::BadRequest("Invalid data"))?; // TODO Use given error.to_string()

        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        db_connection
            .transaction(|| {
                #[derive(Insertable)]
                #[table_name = "pixur_authorizations"]
                struct Authorization<'a> {
                    pixur_id: Id30,
                    sub: &'a str,
                }

                let old_recipients: Vec<String> = pixur_authorizations::table
                    .filter(pixur_authorizations::pixur_id.eq(self.id))
                    .select(pixur_authorizations::sub)
                    .load(&*db_connection)?;

                let old_recipients: std::collections::BTreeSet<_> =
                    old_recipients.iter().map(|x| x.as_str()).collect();

                let new_recipients = update_request.metadata.recipients;

                let to_add = new_recipients
                    .difference(&old_recipients)
                    .map(|&sub| Authorization {
                        pixur_id: self.id,
                        sub: sub,
                    })
                    .collect::<Vec<_>>();

                diesel::insert_into(pixur_authorizations::table)
                    .values(&to_add)
                    .execute(&*db_connection)?;

                // TODO (Optionally) send email to recipients in `to_add`

                let to_remove = old_recipients.difference(&new_recipients);

                diesel::delete(
                    pixur_authorizations::table
                        .filter(pixur_authorizations::pixur_id.eq(self.id))
                        .filter(pixur_authorizations::sub.eq_any(to_remove)),
                )
                .execute(&*db_connection)?;

                Ok(Response {
                    status: web::Status::Ok,
                    representations: vec![(
                        MediaType::new("text", "plain", vec!["charset=utf-8".to_string()]),
                        Box::new(move || Box::new("OK") as RepresentationBox) as _,
                    )],
                    cookies: vec![],
                })
            })
            .map_err(|_: diesel::result::Error| HandlingError::InternalServerError)
    }

    async fn async_post(self: Box<Self>, content_type: String, body: hyper::Body) -> Response {
        self.try_post(content_type, body)
            .await
            .unwrap_or_else(|e| e.render())
    }
}

impl Resource for PixuMeta {
    fn get<'a>(self: Box<Self>) -> FutureBox<'a, Response> {
        self.async_get().boxed()
    }

    fn post<'a>(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> FutureBox<'a, Response> {
        self.async_post(content_type, body).boxed()
    }
}

pub struct AuthorizationConsumer {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub id: Id30,
}

impl auth::authorizer::Consumer for AuthorizationConsumer {
    type Authorization = ();

    fn authorization<'a>(self, _: ()) -> Result<Box<dyn Resource + Send + 'static>, Error> {
        Ok(Box::new(PixuMeta {
            db_pool: self.db_pool,
            id: self.id,
        }) as _)
    }
}

// TODO Deduplicate. This is in fact identical to ingest::AuthorizationProvider
pub struct AuthorizationProvider {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl auth::authorizer::Provider for AuthorizationProvider {
    type Authorization = ();

    fn get_authorization(&self, sub: &str) -> Result<Option<Self::Authorization>, web::Error> {
        use diesel::dsl::*;

        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| web::Error::InternalServerError)?;

        let authorized: bool = select(exists(uploaders::table.filter(uploaders::sub.eq(sub))))
            .first::<bool>(&*db_connection)
            .expect("Query must return 1 result");

        if authorized {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}
