use axum::http::StatusCode;
use axum_derive_error::ErrorResponse;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Error, ErrorResponse)]
pub enum ApiError {
    #[error("Project not found.")]
    #[status(StatusCode::NOT_FOUND)]
    ProjectNotFound,

    #[error("Unauthorized.")]
    #[status(StatusCode::UNAUTHORIZED)]
    AuthenticationRequired,

    #[error("This link has expired.")]
    #[status(StatusCode::UNAUTHORIZED)]
    SignatureExpired,

    #[error("Unauthorized.")]
    #[status(StatusCode::UNAUTHORIZED)]
    InvalidSignature,

    #[error("{0}")]
    #[status(StatusCode::BAD_REQUEST)]
    ClientError(String),

    #[error(transparent)]
    ServerError(#[from] anyhow::Error),
}

impl PartialEq for ApiError {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string() && self.status_code() == other.status_code()
    }
}
