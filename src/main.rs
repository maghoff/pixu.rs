#![feature(async_await, await_macro, futures_api)]

#[macro_use] extern crate bart_derive;
#[macro_use] extern crate diesel_migrations;

mod db;

use std::net::SocketAddr;

use futures::prelude::*;
use hyper::http;
use hyper::rt::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "pixu.rs")]
struct Options {
    /// SQLite database file
    #[structopt(name = "DB")]
    db: String,
}

use hyper::{Body, Request, Response};

const TEXT_HTML: &str = "text/html;charset=utf-8";

#[derive(BartDisplay)]
#[template_string="You are looking for {{uri}}\n"]
struct DummyResponse<'a> {
    uri: &'a http::uri::Uri,
}

enum Error {
    BadRequest,
}

trait Resource {
    fn etag(&self) -> Option<http::ETag>;
    fn last_modified(&self) -> Option<Timestamp>;
}

trait QueryableResource {
    fn query(self: Box<Self>, query: Option<&str>) -> Result<Box<dyn Resource>, Error>;
}

async fn lookup(_path: &str) -> Box<dyn QueryableResource> {
    // if _path == "*" { asterisk_resource() }

    unimplemented!()
}

async fn handle_request_core(req: Request<Body>) ->
    Result<Response<Body>, Error>
{
    let queryable_resource = await!(lookup(&req.uri().path()));
    let resource = queryable_resource.query(req.uri().query())?;

    if let Some(etag) = resource.etag() {
        // Check ETag-related If-headers
        unimplemented!();
    }

    if let Some(last_modified) = resource.last_modified() {
        // Check last_modified-related If-headers
        unimplemented!();
    }

    unimplemented!()
}

async fn handle_request(req: Request<Body>) ->
    Result<Response<Body>, Box<std::error::Error + Send + Sync + 'static>>
{
    match await!(handle_request_core(req)) {
        Ok(res) => Ok(res),
        Err(Error::BadRequest) => {
            let body = DummyResponse { uri: req.uri(), };

            Ok(Response::builder()
                .header(http::header::CONTENT_TYPE, TEXT_HTML)
                .body(Body::from(body.to_string()))
                .unwrap()
            )
        }
    }
}

fn main() -> Result<(), Box<std::error::Error>>{
    let opt = Options::from_args();
    let _db = db::create_pool(opt.db)?;

    let bind_host = "127.0.0.1".parse().expect("Acceptable IP address");
    let bind_port = 1212;

    let service_fn = || {
        hyper::service::service_fn(
            |req| handle_request(req).boxed().compat()
        )
    };

    let server =
        hyper::server::Server::bind(&SocketAddr::new(bind_host, bind_port))
            .serve(service_fn);

    println!("Listening on http://{}", server.local_addr());

    tokio::run(server.map_err(|e| {
        eprintln!("server error: {}", e);
    }));

    Ok(())
}
