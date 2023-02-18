use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    axum::state::AppState,
    http::controllers::{ProjectController, TeamController},
};

pub fn mount() -> Router<AppState> {
    Router::new().nest(
        "/team/:team",
        Router::new()
            .route("/", get(TeamController::show))
            .route("/projects", post(ProjectController::store)),
    )
}
