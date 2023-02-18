use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{axum::state::AppState, http::controllers::ProjectController};

pub fn mount() -> Router<AppState> {
    Router::new().nest(
        "/project/:project",
        Router::new()
            .route("/", get(ProjectController::show))
            .route("/", post(ProjectController::update))
            .route("/", delete(ProjectController::delete)),
    )
}
