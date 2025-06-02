#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("user not found")]
    NotFound,

    #[error("malformed record: {0}")]
    MalformedRecord(String),

    #[error("internal error: {0}")]
    Internal(String),
}
