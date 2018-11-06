extern crate structopt;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "pixu.rs")]
struct Options {
    /// SQLite database file
    #[structopt(name = "DB")]
    db: String,
}

fn main() {
    let opt = Options::from_args();
    println!("Hello, {}!", opt.db);
}
