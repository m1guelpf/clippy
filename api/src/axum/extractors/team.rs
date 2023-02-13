use std::collections::HashMap;

use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::request::Parts,
    RequestPartsExt,
};

use crate::{
    axum::{errors::ApiError, state::AppState},
    prisma::{team, user},
};

use super::User;

#[allow(clippy::module_name_repetitions)]
pub struct TeamForUser(pub team::Data);

#[async_trait]
impl FromRequestParts<AppState> for TeamForUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let User(user) = parts.extract_with_state::<User, AppState>(state).await?;

        let Path(path) = parts
            .extract::<Path<HashMap<String, String>>>()
            .await
            .unwrap();

        let path = path
            .get("team")
            .ok_or_else(|| ApiError::ClientError("Missing team ID".to_string()))?
            .clone();

        let team = state
            .prisma
            .team()
            .find_first(vec![
                team::id::equals(path),
                team::members::some(vec![user::id::equals(user.id)]),
            ])
            .with(team::projects::fetch(vec![]))
            .exec()
            .await;

        match team {
            Ok(Some(team)) => Ok(Self(team)),
            _ => Err(ApiError::ProjectNotFound),
        }
    }
}
