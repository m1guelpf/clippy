use anyhow::Context;
use axum::extract::State;
use axum_jsonschema::Json;

use crate::{
    axum::{
        errors::ApiResult,
        extractors::{TeamForUser, User},
        state::AppState,
    },
    prisma::{team, user},
};

pub async fn index(
    User(user): User,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<team::Data>>> {
    let teams = state
        .prisma
        .team()
        .find_many(vec![team::members::some(vec![user::id::equals(user.id)])])
        .exec()
        .await
        .context("Failed to get teams")?;

    Ok(Json(teams))
}

#[allow(clippy::unused_async)]
pub async fn show(TeamForUser(team): TeamForUser) -> Json<team::Data> {
    Json(team)
}
