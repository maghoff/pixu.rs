#[macro_use] extern crate diesel_migrations;
extern crate diesel;
extern crate r2d2_diesel;
extern crate r2d2;
extern crate structopt;

mod db;

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

    Ok(())
}
