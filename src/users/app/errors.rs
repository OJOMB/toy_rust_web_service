use crate::users::service::errors;
use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Serialize)]
struct Error {
    message: String,
}

pub fn from_service_error(svc_err: errors::Error) -> HttpResponse {
    match svc_err {
        errors::Error::NotFound => {
            let err = Error {
                message: svc_err.to_string(),
            };

            HttpResponse::NotFound()
                .content_type("application/json")
                .body(serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()))
        }
        errors::Error::Validation(e) | errors::Error::MissingParameters(e) => {
            let err = Error { message: e };

            HttpResponse::BadRequest()
                .content_type("application/json")
                .body(serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()))
        }
        errors::Error::ConflictingUser(_) => {
            let err = Error {
                message: svc_err.to_string(),
            };

            HttpResponse::Conflict()
                .content_type("application/json")
                .body(serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()))
        }
        _ => {
            tracing::error!("Unhandled service error: {:?}", svc_err);

            let err = Error {
                message: format!("Internal error"),
            };

            HttpResponse::InternalServerError()
                .content_type("application/json")
                .body(serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()))
        }
    }
}
