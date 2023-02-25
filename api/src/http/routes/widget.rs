use axum::{
    routing::{get, post},
    Router,
};

use crate::{axum::state::AppState, http::controllers::WidgetController};

pub fn mount() -> Router<AppState> {
    Router::new().nest(
        "/widget",
        Router::new()
            .route("/", get(WidgetController::show))
            .route("/search", post(WidgetController::search))
            .route("/stream", post(WidgetController::stream)),
    )
}
