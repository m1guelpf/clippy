use std::collections::HashMap;

use anyhow::Context;
use axum::{
    async_trait,
    extract::{FromRequestParts, Path, State},
    http::{request::Parts, HeaderMap},
    RequestPartsExt,
};
use axum_sessions::extractors::ReadableSession;
use url::Url;

use crate::{
    axum::{
        errors::ApiError,
        extractors::{user::SESSION_IDENTIFIER, Origin},
        state::AppState,
    },
    prisma::{self, project, team, user},
};

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
            .context("Missing session")?;

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

        let Origin(mut origin) = parts
            .extract::<Origin>()
            .await
            .map_err(|_| ApiError::ProjectNotFound)?;

        if origin.ends_with("demo.clippy.help") {
            let headers = parts
                .extract::<HeaderMap>()
                .await
                .map_err(|_| ApiError::ClientError("Invalid request".to_string()))?;

            let referer = Url::parse(
                headers
                    .get("referer")
                    .ok_or_else(|| ApiError::ClientError("Invalid request".to_string()))?
                    .to_str()
                    .map_err(|_| ApiError::ClientError("Invalid request".to_string()))?,
            )
            .map_err(|_| ApiError::ClientError("Invalid request".to_string()))?;

            origin = referer
                .path_segments()
                .ok_or_else(|| ApiError::ClientError("Invalid request".to_string()))?
                .next()
                .ok_or_else(|| ApiError::ClientError("Invalid request".to_string()))?
                .to_string();
        }

        let project = state
            .prisma
            .project()
            .find_first(vec![
                project::status::equals(prisma::ProjectStatus::Trained),
                project::WhereParam::Origins(prisma::read_filters::JsonFilter::ArrayContains(
                    Some(origin.into()),
                )),
            ])
            .exec()
            .await;

        match project {
            Ok(Some(project)) => Ok(Self(project)),
            _ => Err(ApiError::ProjectNotFound),
        }
    }
}
