#[macro_use] extern crate diesel_migrations;
extern crate diesel;
extern crate hyper;
extern crate r2d2_diesel;
extern crate r2d2;
extern crate structopt;
extern crate tokio;

mod db;

use std::net::SocketAddr;
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
        use hyper::{Body, Response, service::service_fn_ok};
        service_fn_ok(|_req| {
            Response::new(Body::from("Hello World"))
        })
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
