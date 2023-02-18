use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_jsonschema::Json;
use axum_sessions::extractors::WritableSession;
use chrono::Duration;
use map_macro::map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    axum::{
        errors::{ApiError, ApiResult},
        extractors::{signed_url, user::SESSION_IDENTIFIER, SignedUrl},
        state::AppState,
    },
    prisma::user,
    utils::email,
};

#[derive(Debug, Deserialize, Validate, JsonSchema)]
pub struct MagicLoginRequest {
    #[validate(email)]
    email: String,
}

pub async fn magic_login(
    _: SignedUrl,
    mut session: WritableSession,
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult<Redirect> {
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
        .insert(SESSION_IDENTIFIER, user.id)
        .map_err(|_| ApiError::ServerError("Could not insert user_id into session".into()))?;

    Ok(Redirect::to("https://clippy.help/dashboard"))
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StatusResponse {
    message: &'static str,
}

pub async fn request_link(Json(req): Json<MagicLoginRequest>) -> ApiResult<Json<StatusResponse>> {
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

    Ok(Json(StatusResponse {
        message: "Email sent",
    }))
}

#[allow(clippy::unused_async)]
pub async fn logout(mut session: WritableSession) -> Json<StatusResponse> {
    session.remove(SESSION_IDENTIFIER);

    Json(StatusResponse {
        message: "Logged out",
    })
}
