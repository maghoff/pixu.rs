use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::{compat::Stream01CompatExt, TryStreamExt};
use lettre::{SmtpTransport, Transport};
use lettre_email::{EmailBuilder, Mailbox};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use web::{Error, Get, MediaType, Post, RepresentationBox, Resource, Response};

use super::auth;
use super::auth_provider;
use super::handling_error::HandlingError;
use crate::db::schema::*;
use crate::id30::Id30;

pub struct PixurMeta {
    title: String,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    id: Id30,
    base_url: String,
    mailer: std::sync::Arc<std::sync::Mutex<SmtpTransport>>,
    sender: Mailbox,
}

#[derive(serde_derive::Serialize)]
struct MetadataGet {
    recipients: Vec<String>,

    crop_left: f32,
    crop_right: f32,
    crop_top: f32,
    crop_bottom: f32,
}

#[derive(serde_derive::Deserialize)]
struct MetadataPost<'a> {
    #[serde(borrow)]
    recipients: std::collections::BTreeSet<&'a str>,

    crop_left: Option<f32>,
    crop_right: Option<f32>,
    crop_top: Option<f32>,
    crop_bottom: Option<f32>,
}

#[derive(serde_derive::Deserialize)]
struct EmailDetails<'a> {
    title: &'a str,
    message: &'a str,
}

#[derive(serde_derive::Deserialize)]
struct UpdateRequest<'a> {
    #[serde(borrow)]
    metadata: MetadataPost<'a>,

    #[serde(borrow)]
    send_email: Option<EmailDetails<'a>>,
}

fn implicit_pixur_series(
    pixur_id: Id30,
    db_connection: &SqliteConnection,
) -> Result<Option<Id30>, HandlingError> {
    use diesel::sql_types::Integer;

    #[derive(QueryableByName)]
    struct IdRow {
        #[sql_type = "Integer"]
        id: Id30,
    }

    let pixur_series: Vec<IdRow> = diesel::dsl::sql_query(
        "\
            SELECT id \
            FROM pixur_series \
            JOIN ( \
                SELECT id AS singleton_id \
                FROM pixur_series \
                GROUP BY id \
                HAVING COUNT(*) = 1 \
            ) \
            ON pixur_series.id = singleton_id \
            WHERE pixurs_id = ? \
            LIMIT 1 \
        ",
    )
    .bind::<diesel::sql_types::Integer, _>(pixur_id)
    .load(db_connection)
    .map_err(|_| HandlingError::InternalServerError)?;

    Ok(pixur_series.get(0).map(|x| x.id))
}

impl PixurMeta {
    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        // Backwards compatibility for implicitly shared single pixur:
        let recipients: Vec<String> = match implicit_pixur_series(self.id, &*db_connection)? {
            Some(pixur_series_id) => pixur_series_authorizations::table
                .filter(pixur_series_authorizations::pixur_series_id.eq(pixur_series_id))
                .select(pixur_series_authorizations::sub)
                .load(&*db_connection)
                .map_err(|_| HandlingError::InternalServerError)?,
            None => vec![],
        };

        let (crop_left, crop_right, crop_top, crop_bottom) = pixurs::table
            .filter(pixurs::id.eq(self.id))
            .select((
                pixurs::crop_left,
                pixurs::crop_right,
                pixurs::crop_top,
                pixurs::crop_bottom,
            ))
            .first(&*db_connection)
            .map_err(|_| HandlingError::InternalServerError)?;

        let metadata = MetadataGet {
            recipients,
            crop_left,
            crop_right,
            crop_top,
            crop_bottom,
        };

        let json =
            serde_json::to_string(&metadata).map_err(|_| HandlingError::InternalServerError)?;

        Ok(Response::new(
            web::Status::Ok,
            vec![(
                MediaType::new("application", "json", vec![]),
                Box::new(move || Box::new(json) as RepresentationBox),
            )],
        ))
    }

    fn send_email_notification(&self, email_details: &EmailDetails, recipients: &[&str]) {
        #[derive(BartDisplay)]
        #[template = "templates/notification-email.html"]
        struct HtmlMail<'a> {
            title: &'a str,
            message: &'a str,
            url: &'a str,
        }

        let mut mailer = self
            .mailer
            .lock()
            .expect("Don't know what to do about Poison");

        let url = format!("{}{}", self.base_url, self.id);

        let html_body = HtmlMail {
            title: email_details.title,
            message: email_details.message,
            url: &url,
        }
        .to_string();

        let text_body = format!(
            "Hei 😊\n\n{}\n\nÅpne bildet på magnusogdisa.no: {}",
            email_details.message, url
        );

        for email in recipients {
            let email = EmailBuilder::new()
                .to(*email)
                .from(self.sender.clone())
                .subject(email_details.title)
                .alternative(&html_body, &text_body)
                .build()
                .unwrap();

            mailer.send(email.into()).unwrap();

            // TODO How to handle errors here?
        }
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
                // TODO Implement backwards compat for sharing
                /*
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

                if let Some(email_details) = update_request.send_email {
                    self.send_email_notification(
                        &email_details,
                        &to_add.iter().map(|x| x.sub).collect::<Vec<_>>(),
                    );
                }

                let to_remove = old_recipients.difference(&new_recipients);

                diesel::delete(
                    pixur_authorizations::table
                        .filter(pixur_authorizations::pixur_id.eq(self.id))
                        .filter(pixur_authorizations::sub.eq_any(to_remove)),
                )
                .execute(&*db_connection)?;
                */

                #[derive(AsChangeset)]
                #[table_name = "pixurs"]
                struct UpdateCrop {
                    crop_left: Option<f32>,
                    crop_right: Option<f32>,
                    crop_top: Option<f32>,
                    crop_bottom: Option<f32>,
                }

                diesel::update(pixurs::table.filter(pixurs::id.eq(self.id)))
                    .set(UpdateCrop {
                        crop_left: update_request.metadata.crop_left,
                        crop_right: update_request.metadata.crop_right,
                        crop_top: update_request.metadata.crop_top,
                        crop_bottom: update_request.metadata.crop_bottom,
                    })
                    .execute(&*db_connection)
                    .or_else(|err| match err {
                        // When there are no changes, QueryBuilderError results:
                        diesel::result::Error::QueryBuilderError(_) => Ok(0),
                        err => Err(err),
                    })?;

                Ok(Response {
                    status: web::Status::Ok,
                    representations: vec![(
                        MediaType::new("text", "plain", vec!["charset=utf-8".to_string()]),
                        Box::new(move || Box::new("OK") as RepresentationBox),
                    )],
                    cookies: vec![],
                })
            })
            .map_err(|e: diesel::result::Error| {
                dbg!(e);
                HandlingError::InternalServerError
            })
    }
}

#[async_trait::async_trait]
impl Get for PixurMeta {
    async fn representations(self: Box<Self>) -> Response {
        let title = self.title.clone();

        self.try_get().await.unwrap_or_else(|e| e.render(&title))
    }
}

#[async_trait::async_trait]
impl Post for PixurMeta {
    async fn post(self: Box<Self>, content_type: String, body: hyper::Body) -> Response {
        let title = self.title.clone();

        self.try_post(content_type, body)
            .await
            .unwrap_or_else(|e| e.render(&title))
    }
}

pub struct AuthorizationConsumer {
    pub title: String,
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub id: Id30,
    pub base_url: String,
    pub mailer: std::sync::Arc<std::sync::Mutex<SmtpTransport>>,
    pub sender: Mailbox,
}

impl auth::authorizer::Consumer for AuthorizationConsumer {
    type Authorization = auth_provider::CanEdit;

    fn authorization(self, _: Self::Authorization) -> Result<Resource, Error> {
        Ok(Resource {
            etag: None,
            get: Some(Box::new(PixurMeta {
                title: self.title.clone(),
                db_pool: self.db_pool.clone(),
                id: self.id,
                base_url: self.base_url.clone(),
                mailer: self.mailer.clone(),
                sender: self.sender.clone(),
            })),
            post: Some(Box::new(PixurMeta {
                //FIXME Curious duplication of get and post
                title: self.title,
                db_pool: self.db_pool,
                id: self.id,
                base_url: self.base_url,
                mailer: self.mailer,
                sender: self.sender,
            })),
        })
    }
}
