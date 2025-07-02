use crate::users::service::errors;
use actix_web::{HttpResponse, error::JsonPayloadError};
use serde::Serialize;
use serde_json::json;

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

pub fn json_error_handler(
    err: JsonPayloadError,
    _req: &actix_web::HttpRequest,
) -> actix_web::Error {
    let message = match &err {
        JsonPayloadError::ContentType => "Invalid content type".to_string(),
        JsonPayloadError::Deserialize(json_err) => {
            // You can parse the specific field error here if desired
            json_err.to_string()
        }
        _ => "Invalid JSON payload".to_string(),
    };

    let error_response = HttpResponse::BadRequest()
        .content_type("application/json")
        .body(json!({ "message": message }).to_string());

    actix_web::error::InternalError::from_response(err, error_response).into()
}
