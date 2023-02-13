use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use axum_jsonschema::Json;
use axum_sessions::extractors::WritableSession;
use chrono::Duration;
use map_macro::map;
use schemars::JsonSchema;
use serde_json::json;
use validator::Validate;

use crate::{
    axum::{
        errors::{ApiError, ApiResult},
        extractors::{signed_url, SignedUrl},
        state::AppState,
    },
    prisma::user,
    utils::email,
};

pub fn mount() -> Router<AppState> {
    Router::new()
        .route("/", get(magic_login))
        .route("/", post(request_link))
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Validate, JsonSchema)]
struct MagicLoginRequest {
    #[validate(email)]
    email: String,
}

async fn magic_login(
    _: SignedUrl,
    mut session: WritableSession,
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult<impl IntoResponse> {
    let email = query
        .get("email")
        .ok_or_else(|| ApiError::ClientError("No email provided".into()))?
        .to_string();

    let user = state
        .prisma
        .user()
        .find_unique(user::email::equals(email))
        .exec()
        .await;

    let Ok(Some(user)) = user else {
        return Err(ApiError::ClientError("User not found".into()));
    };

    session
        .insert("user_id", user.id)
        .map_err(|_| ApiError::ServerError("Could not insert user_id into session".into()))?;

    Ok(Redirect::to("https://clippy.help/dashboard"))
}

async fn request_link(Json(req): Json<MagicLoginRequest>) -> ApiResult<impl IntoResponse> {
    let link = signed_url::build(
        "/auth/login",
        map! { "email" => req.email.as_ref() },
        Some(Duration::days(1)),
    );

    let message = email::from_template("magic-link", map! { "link" => link })
        .to(req.email)
        .build();

    email::send(message)
        .await
        .map_err(|_| ApiError::ServerError("Could not send email".into()))?;

    Ok(Json(json!({ "message": "Email sent" })))
}
