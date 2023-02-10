use axum::{async_trait, extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_sessions::extractors::ReadableSession;

use crate::{
    axum::{errors::ApiError, state::AppState},
    prisma::user,
};

pub struct User(pub user::Data);

#[async_trait]
impl FromRequestParts<AppState> for User {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session = parts.extract::<ReadableSession>().await.unwrap();

        let user_id = session
            .get("user_id")
            .ok_or(ApiError::AuthenticationRequired)?;

        let user = state
            .prisma
            .user()
            .find_unique(user::UniqueWhereParam::IdEquals(user_id))
            .exec()
            .await;

        match user {
            Ok(Some(user)) => Ok(Self(user)),
            _ => Err(ApiError::AuthenticationRequired),
        }
    }
}
