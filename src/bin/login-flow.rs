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
    ValidationA,
    ValidationB,
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
    let claims_a = Claims {
        phase: AuthPhase::ValidationA,
        sub: email.clone(),
        exp: NumberDate::from(chrono::Utc::now() + chrono::Duration::hours(2)),
        jti: rand::random(),
    };
    let token_a = jsonwebtoken::encode(&Header::default(), &claims_a, KEY).unwrap();
    println!("{}", token_a);

    let claims_b = Claims {
        phase: AuthPhase::ValidationB,
        ..claims_a
    };

    let token_b = jsonwebtoken::encode(&Header::default(), &claims_b, KEY).unwrap();
    println!("{}", token_b);
}

fn verify(token_a: &str, token_b: &str) -> Result<(), Box<dyn std::error::Error>> {
    let claims_a = jsonwebtoken::decode::<Claims>(
        &token_a,
        KEY,
        &Validation {
            algorithms: vec![Algorithm::HS256],
            ..Default::default()
        },
    )?.claims;

    let claims_b = jsonwebtoken::decode::<Claims>(
        &token_b,
        KEY,
        &Validation {
            algorithms: vec![Algorithm::HS256],
            ..Default::default()
        },
    )?.claims;

    let phases_ok = claims_a.phase == AuthPhase::ValidationA && claims_b.phase == AuthPhase::ValidationB;
    let sub_ok = claims_a.sub == claims_b.sub;
    let jti_ok = claims_a.jti == claims_b.jti;

    let ok = phases_ok && sub_ok && jti_ok;

    let login = if ok {
        Ok(claims_a.sub)
    } else {
        Err("Validation token mismatch")
    }?;

    println!("Valid login for {}", login);

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
