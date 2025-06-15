use axum::http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{message}")]
    ValidationError { message: String, status: StatusCode },

    #[error("DB error: {0}")]
    DbError(String),

    #[error("Unexpected error: {0}")]
    Unexpected(String),

    #[error("error establishing server: {0}")]
    EstablishServer(String),

    #[error("error initializing Config: {0}")]
    ConfigError(String),
    
    #[error("error parsing env value: {0}")]
    EnvError(String),
}
