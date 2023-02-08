use std::collections::HashMap;

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, State},
    http::request::Parts,
    RequestPartsExt,
};

use crate::{
    axum::{errors::ApiError, state::AppState},
    prisma::project,
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
            .find_unique(project::UniqueWhereParam::IdEquals(path))
            .exec()
            .await;

        match project {
            Ok(Some(project)) => Ok(Self(project)),
            _ => Err(ApiError::ProjectNotFound),
        }
    }
}
