use crate::web::{QueryableResource, Representation, MediaType};

pub async fn lookup(path: &str) -> Box<dyn QueryableResource> {
    let path = path.to_string();

    Box::new(vec![(
        MediaType::new("text", "html", vec![ "charset=utf-8".to_string() ]),
        Box::new(move || {
            #[derive(BartDisplay)]
            #[template_string="You are looking for {{path}}\n"]
            struct DummyResponse<'a> {
                path: &'a str,
            }

            Box::new(DummyResponse { path: &path }.to_string()) as Box<dyn Representation>
        }) as Box<dyn FnOnce() -> Box<dyn Representation>>
    )]) as _
}
