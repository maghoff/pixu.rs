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

    let site = site::Site::new(db_pool);

    let service_fn =
        || hyper::service::service_fn(|req| web::handle_request(&site, req).boxed().compat());

    let server =
        hyper::server::Server::bind(&SocketAddr::new(bind_host, bind_port)).serve(service_fn);

    println!("Listening on http://{}", server.local_addr());

    // The following implicitly starts a thread pool which in turn blocks
    // propagation of panics. I'm not sure I want to deal with panics that
    // way yet. Also, I can't get it working with the site borrow. Hm...

    // use futures::compat::Future01CompatExt;
    // tokio::run(
    //     server
    //         .compat()
    //         .map_err(|e| eprintln!("server error: {}", e))
    //         .boxed()
    //         .compat(),
    // );

    // Alternative: Start a tokio core that's limited to the current thread
    use tokio::runtime::current_thread::Runtime;
    let mut runtime = Runtime::new().unwrap();
    runtime
        .block_on(server)
        .map_err(|e| format!("server error: {}", e))?;

    Ok(())
}
