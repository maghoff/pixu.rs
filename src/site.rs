use core::future::Future;
use std::pin::Pin;

use futures::future::FutureExt;
use hyper::http;
use web::{QueryableResource, Resource, Representation, MediaType, Lookup};

struct Index;

impl Resource for Index {
    fn get(self: Box<Self>) ->
        (http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + 'static> + 'static>)>)
    {
        #[derive(BartDisplay)]
        #[template="templates/index.html"]
        struct Template;

        (
            http::StatusCode::OK,
            vec![(
                MediaType::new("text", "html", vec![ "charset=utf-8".to_string() ]),
                Box::new(move || {
                    Box::new(Template.to_string()) as Box<dyn Representation + 'static>
                }) as Box<dyn FnOnce() -> Box<dyn Representation + 'static> + 'static>
            )]
        )
    }

    fn post<'a>(self: Box<Self>) ->
        Pin<Box<dyn Future<Output=(http::StatusCode, Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation + 'static> + 'static>)>)> + Send + 'a>>
    {
        async {
            // let this = self;

            #[derive(BartDisplay)]
            #[template="templates/index-post.html"]
            struct Template<'a> {
                email: &'a str
            }

            (
                http::StatusCode::OK,
                vec![(
                    MediaType::new("text", "html", vec![ "charset=utf-8".to_string() ]),
                    Box::new(move || {
                        Box::new(Template {
                            email: "email",
                        }.to_string()) as Box<dyn Representation + 'static>
                    }) as Box<dyn FnOnce() -> Box<dyn Representation + 'static> + 'static>
                )]
            )
        }.boxed()
    }
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
                Box::new(NotFound.to_string()) as Box<dyn Representation + 'static>
            }) as Box<dyn FnOnce() -> Box<dyn Representation + 'static> + Send + Sync + 'static>
        )]
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
    fn lookup<'a>(&'a self, path: &'a str) ->
        Pin<Box<dyn Future<Output=Box<dyn QueryableResource>> + Send + Sync + 'a>>
    {
        lookup(&path).boxed()
    }
}
