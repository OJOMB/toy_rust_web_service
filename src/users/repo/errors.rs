#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("user not found")]
    NotFound,

    #[error("validation error: {0}")]
    Validation(String),

    #[error("malformed response: {0}")]
    MalformedResponse(String),

    #[error("email address already in use: {0}")]
    EmailAddressAlreadyInUse(String),

    #[error("internal error: {0}")]
    Internal(String),
}
