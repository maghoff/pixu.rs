#[macro_use]
extern crate serde_derive;

use jsonwebtoken::{Algorithm, Header, Validation};
use structopt::StructOpt;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "jwt")]
enum Options {
    Encode { subject: String },
    Decode { jwt: String },
}

fn encode(subject: String) {
    let claims = Claims { sub: subject };

    let token = jsonwebtoken::encode(&Header::default(), &claims, "secret".as_ref()).unwrap();

    println!("{}", token);
}

fn decode(jwt: String) -> Result<(), Box<dyn std::error::Error>> {
    let token_data = jsonwebtoken::decode::<Claims>(
        &jwt,
        "secret".as_ref(),
        &Validation {
            algorithms: vec![Algorithm::HS256],
            validate_exp: false,
            ..Default::default()
        },
    )?;

    println!(
        "Valid token\n{:?}\n{:?}",
        token_data.header, token_data.claims
    );

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use Options::*;

    let opt = Options::from_args();

    match opt {
        Encode { subject } => encode(subject),
        Decode { jwt } => decode(jwt)?,
    };

    Ok(())
}
