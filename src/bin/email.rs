#[macro_use]
extern crate bart_derive;

use structopt::StructOpt;

use lettre::smtp::authentication::{Credentials, Mechanism};
use lettre::smtp::ConnectionReuseParameters;
use lettre::{ClientSecurity, SmtpClient, Transport};

use lettre_email::EmailBuilder;

#[derive(StructOpt)]
struct Params {
    /// SMTP host
    #[structopt(short = "h", long = "host")]
    host: String,

    /// SMTP port
    #[structopt(short = "p", long = "port", default_value = "25")]
    port: u16,

    /// SMTP user name
    #[structopt(short = "u", long = "user")]
    user: String,

    /// SMTP password
    #[structopt(short = "P", long = "pass")]
    password: String,

    /// Recipient name
    #[structopt(long = "rn")]
    recipient_name: String,

    /// Recipient email
    #[structopt(long = "re")]
    recipient_email: String,

    /// Sender name
    #[structopt(long = "sn")]
    sender_name: String,

    /// Sender email
    #[structopt(long = "se")]
    sender_email: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Params::from_args();

    let mut mailer = SmtpClient::new((args.host.as_str(), args.port), ClientSecurity::None)?
        .credentials(Credentials::new(args.user, args.password))
        .smtp_utf8(true)
        .authentication_mechanism(Mechanism::Plain)
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
        .transport();

    #[derive(BartDisplay)]
    #[template = "templates/auth-email.html"]
    struct HtmlMail<'a> {
        title: &'a str,
        url: &'a str,
    }

    let url = "https://magnusogdisa.no/foto/";

    let email = EmailBuilder::new()
        .to((args.recipient_email, args.recipient_name))
        .from((args.sender_email, args.sender_name))
        .subject("Velkommen til magnusogdisa.no 📸")
        .alternative(
            HtmlMail {
                title: "Velkommen til magnusogdisa.no 📸",
                url: &url,
            }
            .to_string(),
            format!("Velkommen 😊\n\nFor å komme til på magnusogdisa.no trenger du bare å følge denne linken:\n\n{}", url),
        )
        .build()?;

    mailer.send(email.into())?;

    Ok(())
}
