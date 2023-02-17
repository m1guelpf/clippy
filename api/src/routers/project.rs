use axum::{
    extract::State,
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use axum_jsonschema::Json;
use schemars::JsonSchema;

use crate::{
    axum::{
        errors::{ApiError, ApiResult},
        extractors::Project,
        state::AppState,
    },
    prisma::{project, ModelType},
    utils::db,
};

pub fn mount() -> Router<AppState> {
    Router::new().nest(
        "/project/:project",
        Router::new()
            .route("/", get(project_info))
            .route("/", post(update_project))
            .route("/", delete(delete_project)),
    )
}

#[allow(clippy::unused_async)]
async fn project_info(Project(project): Project) -> impl IntoResponse {
    Json(project)
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
struct UpdateProjectRequest {
    name: String,
    image_url: String,
    origins: Vec<String>,
    #[serde(with = "db::ModelTypeDef")]
    model_type: ModelType,
}

async fn update_project(
    Project(project): Project,
    State(state): State<AppState>,
    Json(req): Json<UpdateProjectRequest>,
) -> ApiResult<Json<project::Data>> {
    let updated_project = state
        .prisma
        .project()
        .update(
            project::id::equals(project.id),
            vec![
                project::name::set(req.name),
                project::model_type::set(req.model_type),
                project::origins::set(req.origins.into()),
                project::image_url::set(Some(req.image_url)),
            ],
        )
        .exec()
        .await
        .map_err(|_| ApiError::ServerError("Failed to update project.".to_string()))?;

    Ok(Json(updated_project))
}

async fn delete_project(
    Project(project): Project,
    State(state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    state
        .prisma
        .project()
        .delete(project::id::equals(project.id))
        .exec()
        .await
        .map_err(|_| ApiError::ServerError("Failed to delete project.".to_string()))?;

    Ok(())
}
