mod auth;
mod handling_error;
mod image;
mod index;
mod pixu;
mod query_args;
mod thumbnail;

use diesel;
use diesel::sqlite::SqliteConnection;
use futures::task::Spawn;
use futures::FutureExt;
use hyper::http;
use lettre::SmtpTransport;
use lettre_email::Mailbox;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use regex::{Regex, RegexSet};
use std::sync::{Arc, Mutex};
use web::{FutureBox, Lookup, MediaType, QueryHandler, RepresentationBox};

use self::image::Image;
use auth::{InitiateAuth, JwtCookieHandler, VerifyAuthArgsConsumer};
use index::IndexLoader;
use pixu::Pixu;
use thumbnail::Thumbnail;

fn not_found() -> impl QueryHandler {
    // TODO: This one seems to only reply to GET, but should give the same
    // response to the other verbs

    #[derive(BartDisplay)]
    #[template_string = "Not found!\n"]
    struct NotFound;

    (
        http::StatusCode::NOT_FOUND,
        vec![(
            MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || Box::new(NotFound.to_string()) as RepresentationBox) as _,
        )],
    )
}

pub struct Site<S: Spawn + Clone + Send + Sync + 'static> {
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

impl<S: Spawn + Clone + Send + Sync + 'static> Site<S> {
    pub fn new(
        db_pool: Pool<ConnectionManager<SqliteConnection>>,
        mailer: SmtpTransport,
        sender: Mailbox,
        spawn: S,
    ) -> Site<S> {
        Site {
            db_pool,
            mailer: Arc::new(Mutex::new(mailer)),
            sender,
            spawn,
        }
    }

    async fn lookup<'a>(&'a self, path: &'a str) -> Box<dyn QueryHandler + 'static> {
        // TODO Decode URL escapes, keeping in mind that foo%2Fbar is different from foo/bar

        regex_routes! { path,
            _ = r"^$" => Box::new(JwtCookieHandler::new(IndexLoader { db_pool: self.db_pool.clone() })) as _,
            _ = r"^initiate_auth$" => Box::new(InitiateAuth {
                db_pool: self.db_pool.clone(),
                mailer: self.mailer.clone(),
                sender: self.sender.clone(),
                spawn: self.spawn.clone(),
            }) as _,
            _ = r"^verify_auth$" => Box::new(query_args::QueryArgsParser::new(VerifyAuthArgsConsumer)) as _,
            m = r"^(\d+)$" => {
                let id = m[1].parse().unwrap();
                let db = self.db_pool.clone();
                let inner = Pixu::new(db, id);
                Box::new(JwtCookieHandler::new(inner)) as _
            },
            m = r"^thumb/(\d+)$" => {
                let id = m[1].parse().unwrap();
                Box::new(Thumbnail::new(self.db_pool.clone(), id)) as _
            },
            m = r"^img/(\d+)$" => {
                let id = m[1].parse().unwrap();
                Box::new(Image::new(self.db_pool.clone(), id)) as _
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
