use crate::users::repo;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("user not found")]
    NotFound,

    #[error("validation error: {0}")]
    Validation(String),

    #[error("missing parameters: {0}")]
    MissingParameters(String),

    #[error("conflicting user: {0}")]
    ConflictingUser(String),

    #[error("internal error")]
    Internal,
}

impl Error {
    pub fn from_repo_error(repo_err: repo::errors::Error) -> Self {
        match repo_err {
            repo::errors::Error::NotFound => Error::NotFound,
            repo::errors::Error::Validation(e) => Error::Validation(e),
            repo::errors::Error::EmailAddressAlreadyInUse(e) => Error::ConflictingUser(e),
            repo::errors::Error::Internal | repo::errors::Error::MalformedResponse(_) => {
                Error::Internal
            }
        }
    }
}
