use web::{Error, FutureBox, Resource};

pub trait ClaimsConsumer {
    type Claims;

    fn claims<'a>(
        self,
        claims: Option<Self::Claims>,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>>;
}
