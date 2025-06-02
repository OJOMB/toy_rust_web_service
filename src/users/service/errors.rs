use crate::users::repo;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("user not found")]
    NotFound,

    #[error("validation error: {0}")]
    Validation(String),

    #[error("missing parameters: {0}")]
    MissingParameters(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    pub fn from_repo_error(repo_err: repo::errors::Error) -> Self {
        match repo_err {
            repo::errors::Error::NotFound => Error::NotFound,
            repo::errors::Error::Internal(e) | repo::errors::Error::MalformedRecord(e) => {
                Error::Internal(e)
            }
        }
    }
}
