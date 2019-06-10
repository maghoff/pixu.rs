use hyper::http::StatusCode;
use web::{MediaType, RepresentationBox, Response};

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
                StatusCode::BAD_REQUEST,
                vec![(
                    MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                    Box::new(move || {
                        Box::new(BadRequest { details }.to_string()) as RepresentationBox
                    }) as _,
                )],
            ),
            HandlingError::InternalServerError => Response::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                vec![(
                    MediaType::new("text", "html", vec!["charset=utf-8".to_string()]),
                    Box::new(move || Box::new(InternalServerError.to_string()) as RepresentationBox)
                        as _,
                )],
            ),
        }
    }
}
