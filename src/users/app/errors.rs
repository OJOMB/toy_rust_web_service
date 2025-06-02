use crate::users::service;
use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Serialize)]
struct Error {
    message: String,
}

pub fn from_service_error(service_err: service::errors::Error) -> HttpResponse {
    match service_err {
        service::errors::Error::NotFound => {
            let err = Error {
                message: service_err.to_string(),
            };

            HttpResponse::NotFound()
                .content_type("application/json")
                .body(serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()))
        }
        service::errors::Error::Validation(e) => {
            let err = Error { message: e };

            HttpResponse::BadRequest()
                .content_type("application/json")
                .body(serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()))
        }
        _ => {
            let err = Error {
                message: "An unexpected error occurred".to_string(),
            };

            HttpResponse::InternalServerError()
                .content_type("application/json")
                .body(serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()))
        }
    }
}
