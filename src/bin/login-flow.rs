#![feature(async_await)]

#[macro_use]
extern crate serde_derive;

use chrono::{DateTime, Duration, Utc};
use futures::{
    executor::ThreadPool,
    task::{Spawn, SpawnExt},
};
use jsonwebtoken::{Algorithm, Header, Validation};
use std::path::PathBuf;
use structopt::StructOpt;

use lettre::smtp::authentication::{Credentials, Mechanism};
use lettre::smtp::ConnectionReuseParameters;
use lettre::{ClientSecurity, SmtpClient, SmtpTransport, Transport};

use lettre_email::{EmailBuilder, Mailbox};

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
enum Operation {
    Issue { email: String },
    Verify { head_sign: String, claims: String },
}

#[derive(StructOpt, Debug)]
#[structopt(name = "login-flow")]
struct Options {
    #[structopt(parse(from_os_str))]
    config: PathBuf,

    #[structopt(subcommand)]
    operation: Operation,
}

#[derive(Debug, Deserialize)]
struct EmailConfig {
    host: String,
    port: u16,

    user: String,
    password: String,

    sender_name: String,
    sender_email: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    email: EmailConfig,
}

fn is_registered_user(_email: &str) -> bool {
    // TODO implement stub

    true
}

fn maybe_send_email(email: String, claims: String, mut mailer: SmtpTransport, sender: Mailbox) {
    if !is_registered_user(&email) {
        return;
    }

    let base_url = "http://localhost/"; // FIXME

    let redir = "1"; // FIXME
    let verification_link = format!("{}/auth?validation={}&redir={}", base_url, claims, redir);

    let email = EmailBuilder::new()
        .to(email)
        .from(sender)
        .subject("Innlogging")
        .text(format!("Følg denne linken: {}", verification_link))
        .build()
        .unwrap();

    mailer.send(email.into()).unwrap();
}

async fn issue(email: String, mailer: SmtpTransport, sender: Mailbox, mut spawn: impl Spawn) {
    let claims = Claims {
        phase: AuthPhase::Validation,
        sub: email.clone(),
        exp: (Utc::now() + Duration::hours(2)).into(),
        jti: rand::random(),
    };
    let token = jsonwebtoken::encode(&Header::default(), &claims, KEY).unwrap();

    let mut parts = token.split('.');

    let head = parts.next().unwrap();
    let claims = parts.next().unwrap().to_string();
    let sign = parts.next().unwrap();

    // TODO: replace spawn_with_handle with spawn and remove .await. We must
    // .await in this test code, or else the executor would terminate prematurely
    spawn
        .spawn_with_handle(async {
            maybe_send_email(email, claims, mailer, sender);
        })
        .unwrap()
        .await;

    println!("Set-Cookie: let-me-in={}.{}", head, sign);
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

    let config = std::fs::read_to_string(opt.config)?;
    let config: Config = toml::from_str(&config)?;

    let mailer = SmtpClient::new(
        (config.email.host.as_str(), config.email.port),
        ClientSecurity::None,
    )?
    .credentials(Credentials::new(config.email.user, config.email.password))
    .smtp_utf8(true)
    .authentication_mechanism(Mechanism::Plain)
    .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
    .transport();

    let sender: Mailbox = (config.email.sender_email, config.email.sender_name).into();

    let mut executor = ThreadPool::new()?;

    match opt.operation {
        Operation::Issue { email } => {
            executor.run(issue(email, mailer, sender, executor.clone()));
        }
        Operation::Verify { head_sign, claims } => verify(&head_sign, &claims)?,
    };

    Ok(())
}
