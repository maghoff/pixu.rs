use serde::de::DeserializeOwned;

use web::QueryHandler;

pub trait QueryArgsConsumer {
    type Args;

    fn args(self, args: Self::Args) -> Result<Box<dyn web::CookieHandler + Send>, web::Error>;
}

pub struct QueryArgsParser<Consumer, Args>
where
    Consumer: QueryArgsConsumer<Args = Args> + Send,
    Args: DeserializeOwned,
{
    consumer: Consumer,
}

impl<Consumer, Args> QueryArgsParser<Consumer, Args>
where
    Consumer: QueryArgsConsumer<Args = Args> + Send,
    Args: DeserializeOwned,
{
    pub fn new(consumer: Consumer) -> QueryArgsParser<Consumer, Args> {
        QueryArgsParser { consumer }
    }
}

impl<Consumer, Args> QueryHandler for QueryArgsParser<Consumer, Args>
where
    Consumer: QueryArgsConsumer<Args = Args> + Send,
    Args: DeserializeOwned,
{
    fn query(
        self: Box<Self>,
        query: Option<&str>,
    ) -> Result<Box<dyn web::CookieHandler + Send>, web::Error> {
        let args = query.unwrap_or_default();
        let args = serde_urlencoded::from_str(args);

        // TODO Propagate error information
        let args = args.map_err(|_| web::Error::BadRequest)?;

        self.consumer.args(args)
    }
}
