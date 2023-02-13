use axum::{extract::State, routing::get, Router};
use axum_jsonschema::Json;

use crate::{
    axum::{
        errors::{ApiError, ApiResult},
        extractors::User,
        state::AppState,
    },
    prisma::{team, user},
};

mod login;

pub fn mount() -> Router<AppState> {
    Router::new().nest(
        "/auth",
        Router::new()
            .nest("/login", login::mount())
            .route("/user", get(get_user))
            .route("/teams", get(get_teams)),
    )
}

#[allow(clippy::unused_async)]
async fn get_user(User(user): User) -> Json<user::Data> {
    Json(user)
}

#[allow(clippy::unused_async)]
async fn get_teams(
    User(user): User,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<team::Data>>> {
    let teams = state
        .prisma
        .team()
        .find_many(vec![team::members::some(vec![user::id::equals(user.id)])])
        .exec()
        .await
        .map_err(|_| ApiError::ServerError("Could not get teams".into()))?;

    Ok(Json(teams))
}
