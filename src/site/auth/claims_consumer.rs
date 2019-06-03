use web::{Error, FutureBox, Resource};

pub trait ClaimsConsumer {
    type Claims;

    fn authorization<'a>(
        self,
        claims: Self::Claims,
    ) -> FutureBox<'a, Result<Box<dyn Resource + Send + 'static>, Error>>;
}
