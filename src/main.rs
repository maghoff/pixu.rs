#![feature(async_await)]

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

fn main() -> Result<(), Box<std::error::Error>>{
    let opt = Options::from_args();
    let _db = db::create_pool(opt.db)?;

    let bind_host = "127.0.0.1".parse().expect("Acceptable IP address");
    let bind_port = 1212;

    let service_fn = || {
        async {
            use hyper::{Body, Request, Response, service::service_fn_ok};
            Ok(service_fn_ok(|_req: Request<Body>| {
                Response::new(Body::from("Hello World"))
            })) as Result<_, Box<std::error::Error + Send + Sync>>
        }.boxed().compat()
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
