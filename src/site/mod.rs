mod auth;
mod handling_error;
mod id30;
mod image;
mod index;
mod ingest;
mod pixu;
mod query_args;
mod thumbnail;

use diesel;
use diesel::sqlite::SqliteConnection;
use futures::task::Spawn;
use futures::FutureExt;
use lettre::SmtpTransport;
use lettre_email::Mailbox;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use regex::{Regex, RegexSet};
use std::sync::{Arc, Mutex};
use web::{FutureBox, Lookup, MediaType, QueryHandler, RepresentationBox};

use auth::{InitiateAuth, JwtCookieHandler, VerifyAuthArgsConsumer};
use index::IndexLoader;

#[derive(BartDisplay)]
#[template = "templates/layout.html"]
struct Layout<'a> {
    // BartDisplay is unable to parse `dyn`
    #[allow(bare_trait_objects)]
    body: &'a std::fmt::Display,
}

fn not_found() -> impl QueryHandler {
    // TODO: This one seems to only reply to GET, but should give the same
    // response to the other verbs

    #[derive(BartDisplay)]
    #[template_string = "Not found!\n"]
    struct NotFound;

    (
        web::Status::NotFound,
        vec![(
            MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || Box::new(NotFound.to_string()) as RepresentationBox) as _,
        )],
    )
}

fn moved_permanently(redirect: impl Into<String>) -> impl QueryHandler {
    // TODO: This one seems to only reply to GET, but should give the same
    // response to the other verbs

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

    (
        web::Status::MovedPermanently(redirect),
        vec![(
            MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || Box::new(body) as RepresentationBox) as _,
        )],
    )
}

fn static_asset(media_type: MediaType, body: String) -> impl QueryHandler {
    (
        web::Status::Ok,
        vec![(
            media_type,
            Box::new(move || Box::new(body) as RepresentationBox) as _,
        )],
    )
}

pub struct Site<S: Spawn + Clone + Send + Sync + 'static> {
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

use id30::Id30;
fn canonicalize_id30(
    given: &str,
    then: impl Fn(Id30) -> Box<dyn QueryHandler + 'static>,
) -> Box<dyn QueryHandler + 'static> {
    match given.parse::<Id30>() {
        Ok(id) => {
            let canon = id.to_string();
            if given == canon {
                then(id)
            } else {
                Box::new(moved_permanently(canon)) as _
            }
        }
        Err(_) => Box::new(not_found()) as _,
    }
}

impl<S: Spawn + Clone + Send + Sync + 'static> Site<S> {
    pub fn new(
        key: Vec<u8>,
        base_url: String,
        db_pool: Pool<ConnectionManager<SqliteConnection>>,
        mailer: SmtpTransport,
        sender: Mailbox,
        spawn: S,
    ) -> Site<S> {
        Site {
            key,
            base_url,
            db_pool,
            mailer: Arc::new(Mutex::new(mailer)),
            sender,
            spawn,
        }
    }

    async fn lookup<'a>(&'a self, path: &'a str) -> Box<dyn QueryHandler + 'static> {
        // TODO Decode URL escapes, keeping in mind that foo%2Fbar is different from foo/bar

        regex_routes! { path,
            m = r"^([a-zA-Z0-9]{6})$" => {
                // Keep this route on top so it matches first, to notice
                // if introducing other routes that would conflict

                canonicalize_id30(&m[1], |id| {
                    let provider = pixu::AuthorizationProvider { db_pool: self.db_pool.clone(), id };
                    let consumer = pixu::AuthorizationConsumer { db_pool: self.db_pool.clone() };
                    let authorizer = auth::authorizer::Authorizer::new(
                        path.to_string(),
                        provider,
                        consumer,
                    );
                    Box::new(JwtCookieHandler::new(self.key.clone(), authorizer)) as _
                })
            },
            _ = r"^$" => Box::new(
                JwtCookieHandler::new(
                    self.key.clone(),
                    IndexLoader { db_pool: self.db_pool.clone() }
                )
            ) as _,
            _ = r"^style\.css$" => Box::new(static_asset(
                MediaType::new("text", "css", vec!["charset=utf-8".to_string()]),
                include_str!("style.css").to_string(),
            )) as _,
            _ = r"^initiate_auth$" => Box::new(InitiateAuth {
                key: self.key.clone(),
                base_url: self.base_url.clone(),
                db_pool: self.db_pool.clone(),
                mailer: self.mailer.clone(),
                sender: self.sender.clone(),
                spawn: self.spawn.clone(),
            }) as _,
            _ = r"^verify_auth$" => Box::new(query_args::QueryArgsParser::new(VerifyAuthArgsConsumer {
                key: self.key.clone(),
            })) as _,
            m = r"^thumb/([a-zA-Z0-9]{6})$" => {
                canonicalize_id30(&m[1], |id| {
                    let provider = thumbnail::AuthorizationProvider { db_pool: self.db_pool.clone(), id };
                    let consumer = thumbnail::AuthorizationConsumer { db_pool: self.db_pool.clone() };
                    let authorizer = auth::authorizer::Authorizer::new(
                        path.to_string(),
                        provider,
                        consumer,
                    );
                    Box::new(JwtCookieHandler::new(self.key.clone(), authorizer)) as _
                })
            },
            _ = r"^img/$" => {
                let provider = ingest::AuthorizationProvider { db_pool: self.db_pool.clone() };
                let consumer = ingest::AuthorizationConsumer { db_pool: self.db_pool.clone() };
                let authorizer = auth::authorizer::Authorizer::new(
                    path.to_string(),
                    provider,
                    consumer,
                );
                Box::new(JwtCookieHandler::new(self.key.clone(), authorizer)) as _
            },
            m = r"^img/([a-zA-Z0-9]{6})$" => {
                canonicalize_id30(&m[1], |id| {
                    let provider = image::AuthorizationProvider { db_pool: self.db_pool.clone(), id };
                    let consumer = image::AuthorizationConsumer { db_pool: self.db_pool.clone() };
                    let authorizer = auth::authorizer::Authorizer::new(
                        path.to_string(),
                        provider,
                        consumer,
                    );
                    Box::new(JwtCookieHandler::new(self.key.clone(), authorizer)) as _
                })
            },
            ! => Box::new(not_found()) as _
        }
    }
}

impl<S: Spawn + Clone + Send + Sync + 'static> Lookup for Site<S> {
    fn lookup<'a>(&'a self, path: &'a str) -> FutureBox<'a, Box<dyn QueryHandler>> {
        self.lookup(&path).boxed()
    }
}
