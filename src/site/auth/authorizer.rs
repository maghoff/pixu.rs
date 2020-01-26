use futures::future::FutureExt;
use web::{Error, FutureBox, Resource};

#[derive(BartDisplay)]
#[template = "templates/not-authorized.html"]
struct NotAuthorized<'a> {
    claims: &'a Option<super::Claims>,
    self_url: &'a str,
}

fn not_authorized(title: &str, claims: &Option<super::Claims>, self_url: &str) -> web::Response {
    let body = Box::new(
        crate::site::Layout {
            title: title,
            body: &NotAuthorized { claims, self_url },
        }
        .to_string(),
    ) as web::RepresentationBox;

    web::Response::new(
        web::Status::Unauthorized,
        vec![(
            web::MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
            Box::new(move || body) as web::RendererBox,
        )],
    )
}

pub trait Provider {
    type Authorization;

    fn get_authorization(&self, sub: &str) -> Result<Option<Self::Authorization>, Error>;
}

pub trait Consumer {
    type Authorization;

    fn authorization(self, authorization: Self::Authorization) -> Result<Resource, Error>;
}

pub struct Authorizer<P, C, A>
where
    P: Provider<Authorization = A>,
    C: Consumer<Authorization = A>,
{
    title: String,
    provider: P,
    consumer: C,
    self_url: String,
}

impl<P, C, A> Authorizer<P, C, A>
where
    P: Provider<Authorization = A>,
    C: Consumer<Authorization = A>,
{
    pub fn new(title: String, self_url: String, provider: P, consumer: C) -> Self {
        Authorizer {
            title,
            provider,
            consumer,
            self_url,
        }
    }

    fn get_authorization(
        &self,
        claims: &Option<super::Claims>,
    ) -> Result<Option<P::Authorization>, Error> {
        let claims = match claims {
            Some(x) => x,
            None => return Ok(None),
        };

        if claims.phase != super::AuthPhase::LoggedIn {
            return Ok(None);
        }

        self.provider.get_authorization(&claims.sub)
    }

    async fn async_claims(self, claims: Option<super::Claims>) -> Result<Resource, Error> {
        if let Some(auth) = self.get_authorization(&claims)? {
            self.consumer.authorization(auth)
        } else {
            Err(Error::BlanketResponse(not_authorized(
                &self.title,
                &claims,
                &self.self_url,
            )))
        }
    }
}

impl<P, C, A> super::ClaimsConsumer for Authorizer<P, C, A>
where
    P: Provider<Authorization = A> + Send + 'static,
    C: Consumer<Authorization = A> + Send + 'static,
    A: Send + 'static,
{
    type Claims = super::Claims;

    fn claims<'a>(self, claims: Option<Self::Claims>) -> FutureBox<'a, Result<Resource, Error>> {
        self.async_claims(claims).boxed()
    }
}
