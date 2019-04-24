#![feature(async_await, await_macro, futures_api, unsized_locals)]

#[macro_use]
extern crate bart_derive;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate lazy_static;

mod db;
mod site;

use std::net::SocketAddr;

use futures::prelude::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "pixu.rs")]
struct Options {
    /// SQLite database file
    #[structopt(name = "DB")]
    db: String,
}

fn main() -> Result<(), Box<std::error::Error>> {
    let opt = Options::from_args();
    let db_pool = db::create_pool(opt.db)?;

    let bind_host = "127.0.0.1".parse().expect("Acceptable IP address");
    let bind_port = 1212;

    use std::sync::Arc;
    let site = Arc::new(site::Site::new(db_pool));

    let service_fn = move || {
        let site = Arc::clone(&site);
        hyper::service::service_fn(move |req| {
            web::handle_request(Arc::clone(&site), req).boxed().compat()
        })
    };

    let server =
        hyper::server::Server::bind(&SocketAddr::new(bind_host, bind_port)).serve(service_fn);

    println!("Listening on http://{}", server.local_addr());

    // The following implicitly starts a thread pool. This, in turn, blocks
    // propagation of panics..! However, it looks like propagation of panics
    // out to/of tokio::run is planned, see
    //   https://github.com/tokio-rs/tokio/pull/1052

    use futures::compat::Future01CompatExt;
    tokio::run(
        server
            .compat()
            .map_err(|e| eprintln!("server error: {}", e))
            .boxed()
            .compat(),
    );

    Ok(())
}
