use hyper::Body;
use crate::web::{QueryableResource, Resource, Representation, MediaType, Error, ETag, Timestamp};

struct GreeterResource {
    path: String,
}

impl GreeterResource {
    fn new(path: impl ToString) -> Self {
        Self { path: path.to_string() }
    }
}

impl QueryableResource for GreeterResource {
    fn query(self: Box<Self>, _query: Option<&str>)
        -> Result<Box<dyn Resource>, Error>
    {
        Ok(self as _)
    }
}

impl Resource for GreeterResource {
    fn representations(self: Box<Self>)
        -> Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>
    {
        vec![
            (
                MediaType {
                    type_category: "text".to_string(),
                    subtype: "html".to_string(),
                    args: vec![ "charset=utf-8".to_string() ],
                },
                Box::new(move || {
                    self as Box<dyn Representation>
                }) as _
            )
        ]
    }
}

impl Representation for GreeterResource {
    fn etag(&self) -> Option<ETag> { None }
    fn last_modified(&self) -> Option<Timestamp> { None }

    fn body(&self) -> Body {
        #[derive(BartDisplay)]
        #[template_string="You are looking for {{path}}\n"]
        struct DummyResponse<'a> {
            path: &'a str,
        }

        Body::from(DummyResponse { path: &self.path }.to_string())
    }
}

pub async fn lookup(path: &str) -> Box<dyn QueryableResource> {
    Box::new(GreeterResource::new(path)) as _
}
