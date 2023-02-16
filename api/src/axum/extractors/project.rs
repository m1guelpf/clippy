use std::collections::HashMap;

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, State},
    http::request::Parts,
    RequestPartsExt,
};
use axum_sessions::extractors::ReadableSession;

use crate::{
    axum::{errors::ApiError, state::AppState},
    prisma::{self, project, team, user},
};

use super::{user::SESSION_IDENTIFIER, Origin};

pub struct Project(pub project::Data);

#[async_trait]
impl FromRequestParts<AppState> for Project {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let State(state) = parts
            .extract_with_state::<State<AppState>, AppState>(state)
            .await
            .unwrap();

        let session = parts
            .extract::<ReadableSession>()
            .await
            .map_err(|_| ApiError::ServerError("Missing session".to_string()))?;

        let user_id = session
            .get::<String>(SESSION_IDENTIFIER)
            .ok_or(ApiError::AuthenticationRequired)?;

        let Path(path) = parts
            .extract::<Path<HashMap<String, String>>>()
            .await
            .unwrap();

        let path = path
            .get("project")
            .ok_or_else(|| ApiError::ClientError("Missing project ID".to_string()))?
            .clone();

        let project = state
            .prisma
            .project()
            .find_first(vec![
                project::id::equals(path),
                project::team::is(vec![team::members::some(vec![user::id::equals(user_id)])]),
            ])
            .exec()
            .await;

        match project {
            Ok(Some(project)) => Ok(Self(project)),
            _ => Err(ApiError::ProjectNotFound),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct ProjectFromOrigin(pub project::Data);

#[async_trait]
impl FromRequestParts<AppState> for ProjectFromOrigin {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let State(state) = parts
            .extract_with_state::<State<AppState>, AppState>(state)
            .await
            .unwrap();

        let Origin(origin) = parts.extract::<Origin>().await.unwrap();

        let project = state
            .prisma
            .project()
            .find_first(vec![project::WhereParam::Origins(
                prisma::read_filters::JsonFilter::ArrayContains(Some(origin.into())),
            )])
            .exec()
            .await;

        match project {
            Ok(Some(project)) => Ok(Self(project)),
            _ => Err(ApiError::ProjectNotFound),
        }
    }
}
