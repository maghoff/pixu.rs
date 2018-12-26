#![feature(async_await, await_macro, futures_api, never_type, transpose_result, unsized_locals)]

#[macro_use] extern crate bart_derive;
#[macro_use] extern crate diesel_migrations;

mod db;
mod web;
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

fn main() -> Result<(), Box<std::error::Error>>{
    let opt = Options::from_args();
    let _db = db::create_pool(opt.db)?;

    let bind_host = "127.0.0.1".parse().expect("Acceptable IP address");
    let bind_port = 1212;

    let service_fn = || {
        hyper::service::service_fn(
            |req| web::handle_request(req).boxed().compat()
        )
    };

    let server =
        hyper::server::Server::bind(&SocketAddr::new(bind_host, bind_port))
            .serve(service_fn);

    println!("Listening on http://{}", server.local_addr());

    // The following implicitly starts a thread pool which in turn blocks
    // propagation of panics. I'm not sure I want to deal with panics that
    // way yet.
    //
    // tokio::run(server.map_err(|e| {
    //     eprintln!("server error: {}", e);
    // }));

    // Alternative: Start a tokio core that's limited to the current thread
    use tokio::runtime::current_thread::Runtime;
    let mut runtime = Runtime::new().unwrap();
    runtime.block_on(server).map_err(|e| {
        format!("server error: {}", e)
    })?;

    Ok(())
}
