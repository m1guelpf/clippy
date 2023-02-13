use axum::{routing::get, Json, Router};

use crate::{
    axum::{errors::ApiResult, extractors::TeamForUser, state::AppState},
    prisma::team,
};

pub fn mount() -> Router<AppState> {
    Router::new().nest("/team/:team", Router::new().route("/", get(team_info)))
}

#[allow(clippy::unused_async)]
async fn team_info(TeamForUser(team): TeamForUser) -> ApiResult<Json<team::Data>> {
    Ok(Json(team))
}
