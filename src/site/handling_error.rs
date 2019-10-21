use web::{MediaType, RepresentationBox, Response, Status};

#[derive(Debug)]
pub enum HandlingError {
    BadRequest(&'static str),
    InternalServerError,
}

#[derive(BartDisplay)]
#[template = "templates/err/bad-request.html"]
struct BadRequest<'a> {
    details: &'a str,
}

#[derive(BartDisplay)]
#[template = "templates/err/internal-server-error.html"]
struct InternalServerError;

impl HandlingError {
    pub fn render(self) -> Response {
        match self {
            HandlingError::BadRequest(details) => Response::new(
                Status::BadRequest,
                vec![(
                    MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                    Box::new(move || {
                        Box::new(BadRequest { details }.to_string()) as RepresentationBox
                    }) as _,
                )],
            ),
            HandlingError::InternalServerError => Response::new(
                Status::InternalServerError,
                vec![(
                    MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                    Box::new(move || Box::new(InternalServerError.to_string()) as RepresentationBox)
                        as _,
                )],
            ),
        }
    }
}
