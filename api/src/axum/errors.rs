use axum::http::StatusCode;
use axum_derive_error::ErrorResponse;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Error, ErrorResponse, PartialEq, Eq)]
pub enum ApiError {
    #[error("Project not found.")]
    #[status(StatusCode::NOT_FOUND)]
    ProjectNotFound,

    #[error("Unauthorized.")]
    #[status(StatusCode::UNAUTHORIZED)]
    InvalidSignature,

    #[error("{0}")]
    #[status(StatusCode::BAD_REQUEST)]
    ClientError(String),
}
