use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid input from validator: {0}")]
    ValidationError(#[from] garde::Report),

    #[error("DB error: {0}")]
    DbError(String),

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),

    #[error("error establishing server: {0}")]
    EstablishServer(String),

    #[error("error initializing Config: {0}")]
    ConfigError(String),
    
    #[error("error parsing env value: {0}")]
    EnvError(String),

    #[error("error sending request: {0}")]
    SendingRequest(String),
    
    #[error("error user not found: {0}")]
    UserNotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::ConfigError(_) => {self.to_string().into_response()},
            AppError::ValidationError(_) => {
                let msg  = self.to_string();
                (
                    StatusCode::BAD_REQUEST,
                    axum::Json(ApiError {
                        code: StatusCode::BAD_REQUEST.as_u16(),
                        message: msg,
                    })
                    ).into_response()
            },
            AppError::DbError(_) => {
                let msg  = self.to_string();
                (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(ApiError {
                            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                            message: msg,
                        })
                    ).into_response()
            },
            AppError::Unexpected(_) => self.to_string().into_response(),
            AppError::EstablishServer(_) => self.to_string().into_response(),
            AppError::EnvError(_) => self.to_string().into_response(),
            AppError::SendingRequest(_) => self.to_string().into_response(),
            AppError::UserNotFound(_) => self.to_string().into_response(),
        }
    }
}

#[derive(Serialize)]
pub struct ApiError {
    code: u16,
    message: String,
}
