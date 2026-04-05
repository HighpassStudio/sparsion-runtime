use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("event not found: {0}")]
    NotFound(uuid::Uuid),

    #[error("query error: {0}")]
    Query(String),
}
