#[macro_use]
extern crate serde_derive;

use chrono::Utc;
use jsonwebtoken::{Algorithm, Header, Validation};
use structopt::StructOpt;

const KEY: &[u8] = b"secret";

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
struct NumberDate(i64);

impl NumberDate {
    fn _now() -> NumberDate {
        NumberDate(Utc::now().timestamp())
    }

    fn from(datetime: chrono::DateTime<chrono::Utc>) -> NumberDate {
        NumberDate(datetime.timestamp())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum AuthPhase {
    Validation,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    phase: AuthPhase,
    sub: String,
    exp: NumberDate,
    jti: u32,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "login-flow")]
enum Options {
    Issue { email: String },
    Verify { a: String, b: String },
}

fn issue(email: String) {
    let claims = Claims {
        phase: AuthPhase::Validation,
        sub: email.clone(),
        exp: NumberDate::from(chrono::Utc::now() + chrono::Duration::hours(2)),
        jti: rand::random(),
    };
    let token = jsonwebtoken::encode(&Header::default(), &claims, KEY).unwrap();

    let parts = token.rsplitn(2, '.').collect::<Vec<_>>();

    println!("{}\n{}", parts[1], parts[0]);
}

fn verify_core(base_token: &str, signature: &str) -> Result<Claims, Box<dyn std::error::Error>> {
    let token = format!("{}.{}", base_token, signature);

    let token = jsonwebtoken::decode::<Claims>(
        &token,
        KEY,
        &Validation {
            algorithms: vec![Algorithm::HS256],
            ..Default::default()
        },
    )?;

    if token.claims.phase == AuthPhase::Validation {
        Ok(token.claims)
    } else {
        Err("Wrong AuthPhase".into())
    }
}

fn verify(base_token: &str, signature: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Valid login for {}",
        verify_core(base_token, signature)?.sub
    );

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Options::from_args();

    match opt {
        Options::Issue { email } => issue(email),
        Options::Verify { a, b } => verify(&a, &b)?,
    };

    Ok(())
}
