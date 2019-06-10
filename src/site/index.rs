use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::{compat::Stream01CompatExt, FutureExt, TryStreamExt};
use hyper::http;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use serde_urlencoded;
use web::{Cookie, Error, FutureBox, MediaType, RepresentationBox, Resource, Response};

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

#[derive(serde_derive::Deserialize)]
struct PostArgs {
    username: String,
}

#[derive(BartDisplay)]
#[template = "templates/index-post.html"]
struct Post<'a> {
    username: &'a str,
}

impl Index {
    async fn try_post(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> Result<Response, HandlingError> {
        let content_type = content_type;
        if content_type != "application/x-www-form-urlencoded" {
            return Err(HandlingError::BadRequest(
                "Unacceptable Content-Type, must be application/x-www-form-urlencoded",
            ));
        }

        let body = body
            .compat()
            .try_concat()
            .await
            .map_err(|_| HandlingError::InternalServerError)?;
        let args: PostArgs = serde_urlencoded::from_bytes(&body)
            .map_err(|_| HandlingError::BadRequest("Invalid data"))?; // TODO Use given error.to_string()

        use jsonwebtoken::{encode, Header};

        #[derive(serde_derive::Serialize)]
        struct Claims<'a> {
            sub: &'a str,
        }
        let claims = Claims {
            sub: &args.username,
        };

        let token = encode(&Header::default(), &claims, "secret".as_ref()).unwrap();
        let cookie = Cookie::build("let-me-in", token).http_only(true).finish();

        Ok(Response {
            status: http::StatusCode::OK,
            representations: vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(
                        Post {
                            username: &args.username,
                        }
                        .to_string(),
                    ) as RepresentationBox
                }) as _,
            )],
            cookies: vec![cookie],
        })
    }

    async fn post_core(self: Box<Self>, content_type: String, body: hyper::Body) -> Response {
        self.try_post(content_type, body)
            .await
            .unwrap_or_else(|e| e.render())
    }

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

    fn post<'a>(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> FutureBox<'a, Response> {
        self.post_core(content_type, body).boxed()
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
