use std::pin::Pin;

use hyper::http;
use web::{QueryableResource, Representation, MediaType, Lookup};

fn index() -> impl QueryableResource {
    #[derive(BartDisplay)]
    #[template_string="You are looking for {{path}}\n"]
    struct Index<'a> {
        path: &'a str,
    }

    vec![(
        MediaType::new("text", "html", vec![ "charset=utf-8".to_string() ]),
        Box::new(move || {
            Box::new(Index {
                path: "index"
            }.to_string()) as Box<dyn Representation>
        }) as Box<dyn FnOnce() -> Box<dyn Representation>>
    )]
}

fn not_found() -> impl QueryableResource {
    #[derive(BartDisplay)]
    #[template_string="Not found!\n"]
    struct NotFound;

    (
        http::StatusCode::NOT_FOUND,
        vec![(
            MediaType::new("text", "html", vec![ "charset=utf-8".to_string() ]),
            Box::new(move || {
                Box::new(NotFound.to_string()) as Box<dyn Representation>
            }) as Box<dyn FnOnce() -> Box<dyn Representation>>
        )]
    )
}

pub async fn lookup(path: &str) -> Box<dyn QueryableResource> {
    match path {
        "" => Box::new(index()) as _,
        _ => Box::new(not_found()) as _,
    }
}

pub struct Site;

impl Lookup for Site {
    fn lookup<'a>(&'a self, path: &'a str) ->
        Pin<Box<dyn core::future::Future<Output=Box<dyn QueryableResource>> + Send + Sync + 'a>>
    {
        use futures::future::FutureExt;
        lookup(&path).boxed()
    }
}
