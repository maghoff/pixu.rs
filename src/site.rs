use core::future::Future;
use std::pin::Pin;

use futures::future::FutureExt;
use hyper::http;
use serde_urlencoded;
use web::{Lookup, MediaType, QueryableResource, Representation, Resource};

struct Index;

impl Resource for Index {
    fn get(
        self: Box<Self>,
    ) -> (
        http::StatusCode,
        Vec<(
            MediaType,
            Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>,
        )>,
    ) {
        #[derive(BartDisplay)]
        #[template = "templates/index.html"]
        struct Template;

        (
            http::StatusCode::OK,
            vec![(
                MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                Box::new(move || {
                    Box::new(Template.to_string()) as Box<dyn Representation + Send + 'static>
                })
                    as Box<
                        dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static,
                    >,
            )],
        )
    }

    fn post<'a>(
        self: Box<Self>,
        content_type: String,
        body: hyper::Body,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = (
                        http::StatusCode,
                        Vec<(
                            MediaType,
                            Box<
                                dyn FnOnce() -> Box<dyn Representation + Send + 'static>
                                    + Send
                                    + 'static,
                            >,
                        )>,
                    ),
                > + Send
                + 'a,
        >,
    > {
        #[derive(serde_derive::Deserialize)]
        struct Args {
            email: String,
        }

        #[derive(BartDisplay)]
        #[template = "templates/index-post.html"]
        struct Template<'a> {
            email: &'a str,
        }

        async {
            use futures::compat::Stream01CompatExt;
            use futures::TryStreamExt;

            let content_type = content_type;
            if content_type != "application/x-www-form-urlencoded" {
                eprintln!(
                    "Unexpected Content-Type {:?}, parsing as application/x-www-form-urlencoded",
                    content_type
                );
            }

            let body = await! { body.compat().try_concat() }.unwrap(); // TODO Error handling
            let args: Args = serde_urlencoded::from_bytes(&body).unwrap(); // TODO Error handling

            (
                http::StatusCode::OK,
                vec![(
                    MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                    Box::new(move || {
                        Box::new(Template { email: &args.email }.to_string())
                            as Box<dyn Representation + Send + 'static>
                    })
                        as Box<
                            dyn FnOnce() -> Box<dyn Representation + Send + 'static>
                                + Send
                                + 'static,
                        >,
                )],
            )
        }
            .boxed()
    }
}

fn not_found() -> impl QueryableResource {
    #[derive(BartDisplay)]
    #[template_string = "Not found!\n"]
    struct NotFound;

    (
        http::StatusCode::NOT_FOUND,
        vec![(
            MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || {
                Box::new(NotFound.to_string()) as Box<dyn Representation + Send + 'static>
            })
                as Box<dyn FnOnce() -> Box<dyn Representation + Send + 'static> + Send + 'static>,
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
    fn lookup<'a>(
        &'a self,
        path: &'a str,
    ) -> Pin<Box<dyn Future<Output = Box<dyn QueryableResource>> + Send + 'a>> {
        lookup(&path).boxed()
    }
}
