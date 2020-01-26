use async_trait::async_trait;
use web::{Error, Resource};

#[async_trait]
pub trait ClaimsConsumer {
    type Claims;

    async fn claims(self, claims: Option<Self::Claims>) -> Result<Resource, Error>;
}
