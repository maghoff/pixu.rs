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
use regex::RegexSet;
use web::{FutureBox, Lookup, MediaType, QueryableResource, RepresentationBox};

use self::image::Image;
use index::Index;
use pixu::Pixu;
use thumbnail::Thumbnail;

fn not_found() -> impl QueryableResource {
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
    ( $path:expr, $($pat:expr => $res:expr),*, _ => $not:expr ) => {
        lazy_static! {
            static ref ROUTES: RegexSet = RegexSet::new(&[$($pat),*]).unwrap();
        }

        let route = ROUTES.matches($path).into_iter().next();

        let mut i = 0;

        if let Some(r) = route {
            $(
                if r == i {
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

    async fn lookup<'a>(&'a self, path: &'a str) -> Box<dyn QueryableResource + 'static> {
        // TODO Decode URL escapes, keeping in mind that foo%2Fbar is different from foo/bar

        regex_routes! { path,
            r"^$" => Box::new(Index) as _,
            r"^example$" => Box::new(Pixu::new(self.db_pool.clone(), 1)) as _,
            r"^thumb/1$" => Box::new(Thumbnail::new(self.db_pool.clone(), 1)) as _,
            r"^img/1$" => Box::new(Image::new(self.db_pool.clone(), 1)) as _,
            _ => Box::new(not_found()) as _
        }
    }
}

impl Lookup for Site {
    fn lookup<'a>(&'a self, path: &'a str) -> FutureBox<'a, Box<dyn QueryableResource>> {
        self.lookup(&path).boxed()
    }
}
