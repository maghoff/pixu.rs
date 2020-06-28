#[macro_use]
extern crate diesel;

#[path="../id30.rs"]
mod id30;

use std::env;
use rand::{rngs::SmallRng, SeedableRng};
use id30::Id30;

fn main() -> Result<(), &'static str> {
    let arg = env::args().skip(1).next();

    // let id: Id30 = arg.map(|x| x.parse()).or_else(|| Ok(Id30::new(0)))?;
    let id = arg.map(|x| x.parse().map_err(|_| "Parse error"));
    let id = id.unwrap_or_else(|| {
        let mut rng = SmallRng::from_entropy();
        Ok(Id30::new_random(&mut rng))
    })?;

    println!("Id30: {}", id);
    println!("u32: {}", u32::from(id));

    Ok(())
}
