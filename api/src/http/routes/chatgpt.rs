use axum::{
    routing::{get, post},
    Router,
};

use crate::{axum::state::AppState, http::controllers::ChatGPTController};

pub fn mount() -> Router<AppState> {
    Router::new()
        .route(
            "/.well-known/ai-plugin.json",
            get(ChatGPTController::manifesto),
        )
        .route(
            "/.well-known/chatgpt-api.yaml",
            get(ChatGPTController::openapi),
        )
        .route("/chatgpt/search/:project", post(ChatGPTController::search))
}
