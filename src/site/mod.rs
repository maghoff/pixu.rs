mod auth;
mod auth_provider;
mod handling_error;
mod image;
mod index;
mod ingest;
mod pixur_meta;
mod pixur_series;
mod pixur_series_meta;
mod query_args;
mod thumbnail;

use diesel;
use diesel::sqlite::SqliteConnection;
use futures::task::Spawn;
use lettre::SmtpTransport;
use lettre_email::Mailbox;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use regex::{Regex, RegexSet};
use std::sync::{Arc, Mutex};
use web::{Lookup, MediaType, QueryHandler, RepresentationBox, Response};

use auth::{InitiateAuth, JwtCookieHandler, VerifyAuthArgsConsumer};
use index::IndexLoader;

#[derive(BartDisplay)]
#[template = "templates/layout.html"]
struct Layout<'a> {
    title: &'a str,

    // BartDisplay is unable to parse `dyn`
    #[allow(bare_trait_objects)]
    body: &'a std::fmt::Display,
}

fn not_found() -> Response {
    #[derive(BartDisplay)]
    #[template_string = "Not found!\n"]
    struct NotFound;

    Response::new(
        web::Status::NotFound,
        vec![(
            MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || Box::new(NotFound.to_string()) as RepresentationBox),
        )],
    )
}

fn moved_permanently(redirect: impl Into<String>) -> Response {
    let redirect = redirect.into();

    #[derive(BartDisplay)]
    #[template_string = "Moved permanently to {{redirect}}\n"]
    struct MovedPermanently<'a> {
        redirect: &'a str,
    };

    let body = MovedPermanently {
        redirect: &redirect,
    }
    .to_string();

    Response::new(
        web::Status::MovedPermanently(redirect),
        vec![(
            MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || Box::new(body) as RepresentationBox),
        )],
    )
}

struct StaticAsset {
    media_type: MediaType,
    body: String, // Should be Vec<[u8]>, no?
}

#[async_trait::async_trait]
impl web::Get for StaticAsset {
    // TODO permanent cache-control directives?

    async fn representations(self: Box<Self>) -> web::Response {
        let body = Box::new(self.body) as RepresentationBox;

        web::Response::new(
            web::Status::Ok,
            vec![(self.media_type, Box::new(move || body))],
        )
    }
}

fn static_asset(media_type: MediaType, body: String) -> impl QueryHandler {
    web::Resource {
        etag: None,
        get: Some(Box::new(StaticAsset { media_type, body })),
        post: None,
    }
}

pub struct Site<S: Spawn + Clone + Send + Sync + 'static> {
    title: String,
    key: Vec<u8>,
    base_url: String,
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
    mailer: Arc<Mutex<SmtpTransport>>,
    sender: Mailbox,
    spawn: S,
}

macro_rules! regex_routes {
    ( $path:expr, $($m:pat = $pat:expr => $res:expr,)* ! => $not:expr ) => {
        let path = $path;

        lazy_static! {
            static ref ROUTES: RegexSet = RegexSet::new(&[$($pat),*]).unwrap();
        }

        let route = ROUTES.matches(path).into_iter().next();

        let mut i = 0;

        if let Some(r) = route {
            $(
                if r == i {
                    // TODO Avoid reparsing when $m is not used. Somehow.
                    let re = Regex::new($pat).unwrap(); // TODO Memoize
                    let $m = re.captures(path).unwrap();
                    return $res;
                }
                i += 1;
            )*
        }

        let _ = i;

        $not
    };
}

use super::id30::Id30;
fn canonicalize_id30(
    given: &str,
    then: impl Fn(Id30) -> Box<dyn QueryHandler + 'static>,
) -> Result<Box<dyn QueryHandler + 'static>, Response> {
    match given.parse::<Id30>() {
        Ok(id) => {
            let canon = id.to_string();
            if given == canon {
                Ok(then(id))
            } else {
                Err(moved_permanently(canon))
            }
        }
        Err(_) => Err(not_found()),
    }
}

impl<S: Spawn + Clone + Send + Sync + 'static> Site<S> {
    pub fn new(
        title: String,
        key: Vec<u8>,
        base_url: String,
        db_pool: Pool<ConnectionManager<SqliteConnection>>,
        mailer: SmtpTransport,
        sender: Mailbox,
        spawn: S,
    ) -> Site<S> {
        Site {
            title,
            key,
            base_url,
            db_pool,
            mailer: Arc::new(Mutex::new(mailer)),
            sender,
            spawn,
        }
    }

    #[cfg_attr(feature = "dev-server", allow(unreachable_code))]
    async fn lookup<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<Box<dyn QueryHandler + 'static>, Response> {
        // TODO Decode URL escapes, keeping in mind that foo%2Fbar is different from foo/bar

        let title = self.title.clone();

        regex_routes! { path,
            m = r"^([a-zA-Z0-9]{6})$" => {
                // Keep this route on top so it matches first, to notice
                // if introducing other routes that would conflict

                canonicalize_id30(&m[1], |id| {
                    let provider = pixur_series::AuthorizationProvider { db_pool: self.db_pool.clone(), id };
                    let consumer = pixur_series::AuthorizationConsumer { title: title.clone(), db_pool: self.db_pool.clone() };
                    let authorizer = auth::authorizer::Authorizer::new(
                        title.clone(),
                        path.to_string(),
                        provider,
                        consumer,
                    );
                    Box::new(JwtCookieHandler::new(self.key.clone(), authorizer))
                })
            },
            m = r"^([a-zA-Z0-9]{6})/meta$" => {
                // Don't canonicalize URL, or else the trailing /meta would disappear
                // Besides: Users won't type in this URL

                // FIXME: The Id30 in the URL is taken as the ID in the pixurs table,
                // while the parent URL (without trailing /meta) is the ID in pixur_series

                let id = m[1].parse().map_err(|_| not_found())?;
                let provider = auth_provider::CanEditProvider { db_pool: self.db_pool.clone() };
                let consumer = pixur_meta::AuthorizationConsumer {
                    title: title.clone(),
                    db_pool: self.db_pool.clone(),
                    id,
                    base_url: self.base_url.clone(),
                    mailer: self.mailer.clone(),
                    sender: self.sender.clone()
                };
                let authorizer = auth::authorizer::Authorizer::new(
                    title,
                    path.to_string(),
                    provider,
                    consumer,
                );
                Ok(Box::new(JwtCookieHandler::new(self.key.clone(), authorizer)))
            },
            m = r"^([a-zA-Z0-9]{6})/edit$" => {
                // Don't canonicalize URL, or else the trailing /edit would disappear
                // Besides: Users won't type in this URL

                let id = m[1].parse().map_err(|_| not_found())?;
                let provider = auth_provider::CanEditProvider { db_pool: self.db_pool.clone() };
                let consumer = pixur_series_meta::AuthorizationConsumer {
                    title: title.clone(),
                    db_pool: self.db_pool.clone(),
                    id,
                    base_url: self.base_url.clone(),
                    mailer: self.mailer.clone(),
                    sender: self.sender.clone()
                };
                let authorizer = auth::authorizer::Authorizer::new(
                    title,
                    path.to_string(),
                    provider,
                    consumer,
                );
                Ok(Box::new(JwtCookieHandler::new(self.key.clone(), authorizer)))
            },
            _ = r"^$" => Ok(Box::new(
                JwtCookieHandler::new(
                    self.key.clone(),
                    IndexLoader { title, self_url: self.base_url.clone(), db_pool: self.db_pool.clone() }
                )
            )),
            _ = r"^style\.css$" => Ok(Box::new(static_asset(
                MediaType::new("text", "css", vec!["charset=utf-8".to_string()]),
                include_str!("style.css").to_string(),
            ))),
            _ = r"^ingest\.js$" => {
                #[cfg(not(feature = "dev-server"))]
                {
                    Ok(Box::new(static_asset(
                        MediaType::new("text", "javascript", vec!["charset=utf-8".to_string()]),
                        include_str!("../../dist/ingest.js").to_string(),
                    )))
                }

                #[cfg(feature = "dev-server")]
                panic!("ingest.js must be served by the dev server");
            },
            _ = r"^viewer\.js$" => {
                #[cfg(not(feature = "dev-server"))]
                {
                    Ok(Box::new(static_asset(
                        MediaType::new("text", "javascript", vec!["charset=utf-8".to_string()]),
                        include_str!("../../dist/viewer.js").to_string(),
                    )))
                }

                #[cfg(feature = "dev-server")]
                panic!("viewer.js must be served by the dev server");
            },
            _ = r"^initiate_auth$" =>
                Ok(Box::new(web::Resource {
                    etag: None,
                    get: None,
                    post: Some(Box::new(InitiateAuth {
                        title,
                        key: self.key.clone(),
                        base_url: self.base_url.clone(),
                        db_pool: self.db_pool.clone(),
                        mailer: self.mailer.clone(),
                        sender: self.sender.clone(),
                        spawn: self.spawn.clone(),
                    })),
                })),
            _ = r"^verify_auth$" => Ok(Box::new(query_args::QueryArgsParser::new(VerifyAuthArgsConsumer {
                title,
                key: self.key.clone(),
            }))),
            m = r"^thumb/([a-zA-Z0-9]{6})$" => {
                canonicalize_id30(&m[1], |id| {
                    let provider = thumbnail::AuthorizationProvider { db_pool: self.db_pool.clone(), id };
                    let consumer = thumbnail::AuthorizationConsumer { title: title.clone(), db_pool: self.db_pool.clone() };
                    let authorizer = auth::authorizer::Authorizer::new(
                        title.clone(),
                        path.to_string(),
                        provider,
                        consumer,
                    );
                    Box::new(JwtCookieHandler::new(self.key.clone(), authorizer))
                })
            },
            _ = r"^img/$" => {
                let provider = auth_provider::CanEditProvider { db_pool: self.db_pool.clone() };
                let consumer = ingest::AuthorizationConsumer { title: title.clone(), db_pool: self.db_pool.clone(), base_url: self.base_url.clone() };
                let authorizer = auth::authorizer::Authorizer::new(
                    title,
                    path.to_string(),
                    provider,
                    consumer,
                );
                Ok(Box::new(JwtCookieHandler::new(self.key.clone(), authorizer)))
            },
            m = r"^img/([a-zA-Z0-9]{6})$" => {
                canonicalize_id30(&m[1], |id| {
                    let provider = image::AuthorizationProvider { db_pool: self.db_pool.clone(), id };
                    let consumer = image::AuthorizationConsumer { title: title.clone(), db_pool: self.db_pool.clone() };
                    let authorizer = auth::authorizer::Authorizer::new(
                        title.clone(),
                        path.to_string(),
                        provider,
                        consumer,
                    );
                    Box::new(JwtCookieHandler::new(self.key.clone(), authorizer))
                })
            },
            ! => Err(not_found())
        }
    }
}

#[async_trait::async_trait]
impl<S: Spawn + Clone + Send + Sync + 'static> Lookup for Site<S> {
    async fn lookup(&'_ self, path: &'_ str) -> Result<Box<dyn QueryHandler>, Response> {
        self.lookup(&path).await
    }
}
