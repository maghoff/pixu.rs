#[macro_use]
extern crate serde_derive;

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, Header, Validation};
use structopt::StructOpt;

const KEY: &[u8] = b"secret";

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
struct NumberDate(i64);

impl From<DateTime<Utc>> for NumberDate {
    fn from(datetime: DateTime<Utc>) -> Self {
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
    Verify { head_sign: String, claims: String },
}

fn issue(email: String) {
    let claims = Claims {
        phase: AuthPhase::Validation,
        sub: email.clone(),
        exp: (Utc::now() + Duration::hours(2)).into(),
        jti: rand::random(),
    };
    let token = jsonwebtoken::encode(&Header::default(), &claims, KEY).unwrap();

    let mut parts = token.split('.');

    let head = parts.next().unwrap();
    let claims = parts.next().unwrap();
    let sign = parts.next().unwrap();

    println!("head_sign: {}.{}", head, sign);
    println!("claims: {}", claims);
}

fn verify_core(head_sign: &str, claims: &str) -> Result<Claims, Box<dyn std::error::Error>> {
    let mut head_sign = head_sign.splitn(2, '.');
    let head = head_sign.next().unwrap();
    let sign = head_sign.next().ok_or("Missing . in head_sign")?;

    let token = format!("{}.{}.{}", head, claims, sign);

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

fn verify(head_sign: &str, claims: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Valid login for {}", verify_core(head_sign, claims)?.sub);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Options::from_args();

    match opt {
        Options::Issue { email } => issue(email),
        Options::Verify { head_sign, claims } => verify(&head_sign, &claims)?,
    };

    Ok(())
}
