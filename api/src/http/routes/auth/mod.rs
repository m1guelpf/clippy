use axum::{routing::get, Router};

use crate::{
    axum::state::AppState,
    http::controllers::{team as TeamController, user as UserController},
};

mod login;

pub fn mount() -> Router<AppState> {
    Router::new().nest(
        "/auth",
        Router::new()
            .nest("/login", login::mount())
            .route("/user", get(UserController::show))
            .route("/teams", get(TeamController::index)),
    )
}
