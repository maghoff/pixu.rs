use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::{compat::Stream01CompatExt, TryStreamExt};
use lettre::{SmtpTransport, Transport};
use lettre_email::{EmailBuilder, Mailbox};
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use std::borrow::Cow;
use web::{self, Error, MediaType, RepresentationBox, Resource, Response};

use super::auth;
use super::auth_provider;
use super::handling_error::HandlingError;
use crate::comment_position::CommentPosition;
use crate::db::schema::*;
use crate::id30::Id30;

pub struct PixurSeriesMeta {
    title: String,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    id: Id30,
    base_url: String,
    mailer: std::sync::Arc<std::sync::Mutex<SmtpTransport>>,
    sender: Mailbox,
}

#[derive(bart_derive::BartDisplay)]
#[template = "templates/edit-pixur-series.html"]
struct Get<'a> {
    series: &'a [PixurSeriesRow],
}

#[derive(serde_derive::Deserialize)]
struct SeriesRowPost<'a> {
    #[serde(borrow)]
    pixurs_id: Cow<'a, str>, // TODO Impl serde::Deserialize for Id30

    #[serde(borrow)]
    comment: Option<Cow<'a, str>>,

    comment_position: CommentPosition,
}

#[allow(unused)]
#[derive(serde_derive::Deserialize)]
struct EmailDetails<'a> {
    title: &'a str,
    message: &'a str,
}

#[derive(serde_derive::Deserialize)]
struct UpdateRequest<'a> {
    #[serde(borrow)]
    series: Vec<SeriesRowPost<'a>>,
    // #[serde(borrow)]
    // send_email: Option<EmailDetails<'a>>,
}

#[derive(Queryable)]
struct PixurSeriesRow {
    pixurs_id: Id30,
    comment: Option<String>,
    comment_position: CommentPosition,
    average_color: i32,
    thumbs_id: Id30,
}

impl PixurSeriesRow {
    fn average_color(&self) -> String {
        format!("#{:06x}", self.average_color)
    }

    fn position_top(&self) -> bool {
        self.comment_position == CommentPosition::Top
    }

    fn position_center(&self) -> bool {
        self.comment_position == CommentPosition::Center
    }

    fn position_bottom(&self) -> bool {
        self.comment_position == CommentPosition::Bottom
    }
}

impl PixurSeriesMeta {
    async fn try_get(self: Box<Self>) -> Result<Response, HandlingError> {
        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        let series: Vec<PixurSeriesRow> = pixur_series::table
            .inner_join(pixurs::table)
            .select((
                pixur_series::pixurs_id,
                pixur_series::comment,
                pixur_series::comment_position,
                pixurs::average_color,
                pixurs::thumbs_id,
            ))
            .filter(pixur_series::id.eq(self.id))
            .order(pixur_series::order.asc())
            .load(&*db_connection)
            .map_err(|_| HandlingError::InternalServerError)?;

        Ok(Response::new(
            web::Status::Ok,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(
                        super::Layout {
                            title: &self.title,
                            body: &Get { series: &series },
                        }
                        .to_string(),
                    ) as RepresentationBox
                }),
            )],
        ))
    }

    /*
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

        let db_connection = self.db_pool.get().unwrap(); // Not sure how to handle errors

        let series_id = self.id;

        drop(db_connection);

        let url = format!("{}{}", self.base_url, series_id);

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
    */

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

        let update_request: UpdateRequest = serde_json::from_slice(&body)
            .map_err(|_err| HandlingError::BadRequest("Invalid data"))?; // TODO Use given error.to_string()

        let db_connection = self
            .db_pool
            .get()
            .map_err(|_| HandlingError::InternalServerError)?;

        db_connection
            .transaction(|| {
                diesel::delete(pixur_series::table.filter(pixur_series::id.eq(self.id)))
                    .execute(&*db_connection)?;

                let to_add = update_request
                    .series
                    .into_iter()
                    .enumerate()
                    .map(
                        |(
                            order,
                            SeriesRowPost {
                                pixurs_id,
                                comment,
                                comment_position,
                            },
                        )| {
                            (
                                pixur_series::id.eq(self.id),
                                pixur_series::order.eq(order as i32),
                                pixur_series::pixurs_id.eq(pixurs_id.parse::<Id30>().unwrap()),
                                pixur_series::comment.eq(comment),
                                pixur_series::comment_position.eq(comment_position),
                            )
                        },
                    )
                    .collect::<Vec<_>>();

                diesel::insert_into(pixur_series::table)
                    .values(&to_add)
                    .execute(&*db_connection)?;

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
impl web::Get for PixurSeriesMeta {
    async fn representations(self: Box<Self>) -> Response {
        let title = self.title.clone();

        self.try_get().await.unwrap_or_else(|e| e.render(&title))
    }
}

#[async_trait::async_trait]
impl web::Post for PixurSeriesMeta {
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
            get: Some(Box::new(PixurSeriesMeta {
                title: self.title.clone(),
                db_pool: self.db_pool.clone(),
                id: self.id,
                base_url: self.base_url.clone(),
                mailer: self.mailer.clone(),
                sender: self.sender.clone(),
            })),
            post: Some(Box::new(PixurSeriesMeta {
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
