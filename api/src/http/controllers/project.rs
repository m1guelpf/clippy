use axum::extract::State;
use axum_jsonschema::Json;
use lazy_static::lazy_static;
use schemars::JsonSchema;
use serde_json::{json, Value};

use crate::{
    axum::{
        errors::{ApiError, ApiResult},
        extractors::{Project, TeamForUser},
        state::AppState,
    },
    prisma::{project, team, ModelType},
    utils::db,
};

lazy_static! {
    static ref DEFAULT_COPY: Value = json!({
        "title": "Can't find something?",
        "cta": "Ask Clippy your question",
        "loading": "Clippy is thinking...",
        "placeholder": "What do you want to do?",
        "subtitle": "We trained Clippy, an AI assistant, to answer any question from the docs.",
    });
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct Request {
    name: String,
    origins: Vec<String>,
    image_url: Option<String>,
    #[serde(with = "db::ModelTypeDef")]
    model_type: ModelType,
}

#[allow(clippy::unused_async)]
// Get details about a project
pub async fn show(Project(project): Project) -> Json<project::Data> {
    Json(project)
}

// Create a new project for the current team
pub async fn store(
    TeamForUser(team): TeamForUser,
    State(state): State<AppState>,
    Json(req): Json<Request>,
) -> ApiResult<Json<project::Data>> {
    let id = state
        .pika
        .clone()
        .gen("proj")
        .map_err(|_| ApiError::ServerError("Failed to generate project id.".to_string()))?;

    let project = state
        .prisma
        .project()
        .create(
            id,
            req.name,
            DEFAULT_COPY.clone(),
            team::id::equals(team.id),
            vec![
                project::origins::set(req.origins.into()),
                project::image_url::set(req.image_url),
            ],
        )
        .exec()
        .await
        .unwrap();

    Ok(Json(project))
}

pub async fn update(
    Project(project): Project,
    State(state): State<AppState>,
    Json(req): Json<Request>,
) -> ApiResult<Json<project::Data>> {
    let updated_project = state
        .prisma
        .project()
        .update(
            project::id::equals(project.id),
            vec![
                project::name::set(req.name),
                project::image_url::set(req.image_url),
                project::model_type::set(req.model_type),
                project::origins::set(req.origins.into()),
            ],
        )
        .exec()
        .await
        .map_err(|_| ApiError::ServerError("Failed to update project.".to_string()))?;

    Ok(Json(updated_project))
}

pub async fn delete(Project(project): Project, State(state): State<AppState>) -> ApiResult<()> {
    state
        .prisma
        .project()
        .delete(project::id::equals(project.id))
        .exec()
        .await
        .map_err(|_| ApiError::ServerError("Failed to delete project.".to_string()))?;

    Ok(())
}
