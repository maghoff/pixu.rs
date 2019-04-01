mod handling_error;
mod index;

use core::future::Future;
use std::pin::Pin;

use futures::FutureExt;
use hyper::http;
use web::{Lookup, MediaType, QueryableResource, Representation};

use index::Index;

type RepresentationBox = Box<dyn Representation + Send + 'static>;
type FutureBox<'a, Output> = Pin<Box<dyn Future<Output = Output> + Send + 'a>>;

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

pub async fn lookup(path: &str) -> Box<dyn QueryableResource> {
    match path {
        "" => Box::new(Index) as _,
        _ => Box::new(not_found()) as _,
    }
}

pub struct Site;

impl Lookup for Site {
    fn lookup<'a>(&'a self, path: &'a str) -> FutureBox<'a, Box<dyn QueryableResource>> {
        lookup(&path).boxed()
    }
}
