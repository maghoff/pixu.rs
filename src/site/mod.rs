mod auth;
mod handling_error;
mod image;
mod index;
mod pixu;
mod thumbnail;

use diesel;
use diesel::sqlite::SqliteConnection;
use futures::FutureExt;
use hyper::http;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use regex::{Regex, RegexSet};
use web::{FutureBox, Lookup, MediaType, QueryHandler, RepresentationBox};

use self::image::Image;
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

pub struct Site {
    db_pool: Pool<ConnectionManager<SqliteConnection>>,
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

impl Site {
    pub fn new(db_pool: Pool<ConnectionManager<SqliteConnection>>) -> Site {
        Site { db_pool }
    }

    async fn lookup<'a>(&'a self, path: &'a str) -> Box<dyn QueryHandler + 'static> {
        // TODO Decode URL escapes, keeping in mind that foo%2Fbar is different from foo/bar

        regex_routes! { path,
            _ = r"^$" => {
                let auth = auth::JwtCookieHandler::new(IndexLoader);
                Box::new(auth) as _
            },
            _ = r"^example$" => {
                let db = self.db_pool.clone();
                let inner = Pixu::new(db, 1);
                let auth = auth::JwtCookieHandler::new(inner);
                Box::new(auth) as _
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

impl Lookup for Site {
    fn lookup<'a>(&'a self, path: &'a str) -> FutureBox<'a, Box<dyn QueryHandler>> {
        self.lookup(&path).boxed()
    }
}
