#![feature(async_await, futures_api)]

#[macro_use] extern crate bart_derive;
#[macro_use] extern crate diesel_migrations;

mod db;

use std::net::SocketAddr;

use futures::prelude::*;
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

#[derive(BartDisplay)]
#[template_string="You are looking for {{uri}}\n"]
struct DummyResponse<'a> {
    uri: &'a hyper::http::uri::Uri,
}

async fn handle_request(req: Request<Body>) ->
    Result<Response<Body>, Box<std::error::Error + Send + Sync + 'static>>
{
    Ok(Response::new(Body::from(
        DummyResponse { uri: req.uri(), }.to_string()
    )))
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
