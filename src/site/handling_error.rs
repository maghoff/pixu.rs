use web::{MediaType, RepresentationBox, Response, Status};

#[derive(Debug)]
pub enum HandlingError {
    BadRequest(&'static str),
    InternalServerError,
}

#[derive(BartDisplay)]
#[template = "templates/err/bad-request.html"]
struct BadRequest<'a> {
    title: &'a str,
    details: &'a str,
}

#[derive(BartDisplay)]
#[template = "templates/err/internal-server-error.html"]
struct InternalServerError<'a> {
    title: &'a str,
}

impl HandlingError {
    pub fn render(self, title: &str) -> Response {
        match self {
            HandlingError::BadRequest(details) => {
                let body = Box::new(BadRequest { title, details }.to_string()) as RepresentationBox;

                Response::new(
                    Status::BadRequest,
                    vec![(
                        MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                        Box::new(move || body) as _,
                    )],
                )
            }
            HandlingError::InternalServerError => {
                let body = Box::new(InternalServerError { title }.to_string()) as RepresentationBox;

                Response::new(
                    Status::InternalServerError,
                    vec![(
                        MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                        Box::new(move || body) as _,
                    )],
                )
            }
        }
    }
}
