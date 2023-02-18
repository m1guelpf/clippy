use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{axum::state::AppState, http::controllers::AuthController};

pub fn mount() -> Router<AppState> {
    Router::new()
        .route("/", get(AuthController::magic_login))
        .route("/", post(AuthController::request_link))
        .route("/", delete(AuthController::logout))
}
