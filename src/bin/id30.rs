#[macro_use]
extern crate diesel;

#[path = "../id30.rs"]
mod id30;

use id30::Id30;
use rand::{rngs::SmallRng, SeedableRng};
use std::env;

fn main() -> Result<(), &'static str> {
    let arg = env::args().skip(1).next();

    let id = arg.map(|x| x.parse().map_err(|_| "Parse error"));
    let id = id.unwrap_or_else(|| {
        let mut rng = SmallRng::from_entropy();
        Ok(Id30::new_random(&mut rng))
    })?;

    println!("Id30: {}", id);
    println!("u32: {}", u32::from(id));

    Ok(())
}
