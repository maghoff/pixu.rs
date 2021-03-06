#[macro_use]
extern crate bart_derive;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate lazy_static;

mod comment_position;
mod db;
mod id30;
mod image;
mod site;

use std::net::SocketAddr;
use std::path::PathBuf;

use futures::compat::{Executor01CompatExt, Future01CompatExt};
use futures::prelude::*;
use lettre::smtp::authentication::{Credentials, Mechanism};
use lettre::smtp::ConnectionReuseParameters;
use lettre::SmtpClient;
use lettre_email::Mailbox;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "pixu.rs")]
struct Options {
    /// Config file
    #[structopt(parse(from_os_str))]
    config: PathBuf,

    /// SQLite database file
    #[structopt(name = "DB")]
    db: String,
}

#[derive(Debug, serde_derive::Deserialize)]
struct EmailConfig {
    host: String,

    user: String,
    password: String,

    sender_name: String,
    sender_email: String,
}

#[derive(Debug, serde_derive::Deserialize)]
struct Config {
    site_title: String,
    url: String,
    secret: String,
    email: EmailConfig,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Options::from_args();

    let config = std::fs::read_to_string(opt.config)?;
    let config: Config = toml::from_str(&config)?;

    // The following starts a thread pool. This, in turn, blocks propagation
    // of panics..! However, it looks like propagation of panics is planned,
    // see: https://github.com/tokio-rs/tokio/pull/1052
    let mut runtime = tokio::runtime::Runtime::new().expect("failed to start new Runtime");

    let db_pool = db::create_pool(opt.db)?;

    let bind_host = "127.0.0.1".parse().expect("Acceptable IP address");
    let bind_port = 1212;

    let mailer = SmtpClient::new_simple(&config.email.host)?
        .credentials(Credentials::new(config.email.user, config.email.password))
        .smtp_utf8(true)
        .authentication_mechanism(Mechanism::Plain)
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
        .transport();

    let sender: Mailbox = (config.email.sender_email, config.email.sender_name).into();

    let key = base64::decode(&config.secret)?;

    use std::sync::Arc;
    let site = Arc::new(site::Site::new(
        config.site_title, // TODO: Leak this and pass it around as &'static str
        key,
        config.url,
        db_pool,
        mailer,
        sender,
        runtime.executor().compat(),
    ));

    let service_fn = move || {
        let site = Arc::clone(&site);
        hyper::service::service_fn(move |req| {
            web::handle_request(Arc::clone(&site), req).boxed().compat()
        })
    };

    let server =
        hyper::server::Server::bind(&SocketAddr::new(bind_host, bind_port)).serve(service_fn);

    println!("Listening on http://{}", server.local_addr());

    runtime.spawn(
        server
            .compat()
            .map_err(|e| eprintln!("server error: {}", e))
            .boxed()
            .compat(),
    );

    futures::executor::block_on(runtime.shutdown_on_idle().compat()).unwrap();

    Ok(())
}
