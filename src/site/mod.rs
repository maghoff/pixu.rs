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

impl Site {
    pub fn new(db_pool: Pool<ConnectionManager<SqliteConnection>>) -> Site {
        Site { db_pool }
    }

    async fn lookup<'a>(&'a self, path: &'a str) -> Box<dyn QueryableResource + 'static> {
        match path {
            "" => Box::new(Index) as _,
            "example" => Box::new(Pixu::new(self.db_pool.clone(), 1)) as _,
            "thumb/1" => Box::new(Thumbnail::new(self.db_pool.clone(), 1)) as _,
            "img/1" => Box::new(Image::new(self.db_pool.clone(), 1)) as _,
            _ => Box::new(not_found()) as _,
        }
    }
}

impl Lookup for Site {
    fn lookup<'a>(&'a self, path: &'a str) -> FutureBox<'a, Box<dyn QueryableResource>> {
        self.lookup(&path).boxed()
    }
}
