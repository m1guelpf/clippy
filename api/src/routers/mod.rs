use axum::{response::Redirect, routing::get, Json, Router};

mod auth;
mod project;
mod widget;

use crate::axum::{errors::ApiResult, state::AppState};

pub fn mount() -> Router<AppState> {
    Router::new()
        .merge(widget::mount())
        .merge(project::mount())
        .nest("/auth", auth::mount())
        .route("/", get(|| async { Redirect::to("https://clippy.help") }))
        .route("/version", get(version))
}

#[derive(serde::Serialize)]
struct ClippyVersion {
    semver: String,
    rev: Option<String>,
    compile_time: String,
}

#[allow(clippy::unused_async)]
async fn version() -> ApiResult<Json<ClippyVersion>> {
    Ok(Json(ClippyVersion {
        semver: env!("CARGO_PKG_VERSION").to_string(),
        rev: std::env::var("GIT_REV").ok(),
        compile_time: env!("STATIC_BUILD_DATE").to_string(),
    }))
}
