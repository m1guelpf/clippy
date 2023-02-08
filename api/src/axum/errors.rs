use axum::http::StatusCode;
use axum_derive_error::ErrorResponse;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Error, ErrorResponse)]
pub enum ApiError {
    #[error("Project not found.")]
    #[status(StatusCode::NOT_FOUND)]
    ProjectNotFound,

    #[error("{0}")]
    #[status(StatusCode::BAD_REQUEST)]
    ClientError(String),
}
