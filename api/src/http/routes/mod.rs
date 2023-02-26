use axum::{response::Redirect, routing::get, Json, Router};
use std::env;

mod auth;
mod project;
mod team;
mod widget;

use crate::axum::state::AppState;

pub fn mount() -> Router<AppState> {
    Router::new()
        .merge(auth::mount())
        .merge(widget::mount())
        .merge(project::mount())
        .merge(team::mount())
        .route("/version", get(version))
        .route("/", get(|| async { Redirect::to("https://clippy.help") }))
}

#[derive(serde::Serialize)]
struct ClippyVersion {
    semver: String,
    rev: Option<String>,
    compile_time: String,
}

#[allow(clippy::unused_async)]
async fn version() -> Json<ClippyVersion> {
    Json(ClippyVersion {
        rev: env::var("GIT_REV").ok(),
        semver: env!("CARGO_PKG_VERSION").to_string(),
        compile_time: env!("STATIC_BUILD_DATE").to_string(),
    })
}
