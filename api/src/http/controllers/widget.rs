use std::convert::Infallible;

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
};
use axum_jsonschema::Json;
use futures::Stream;
use schemars::JsonSchema;
use serde_json::Value;
use tokio_stream::StreamExt;

use crate::{
    axum::{
        errors::{ApiError, ApiResult},
        extractors::ProjectFromOrigin,
        state::AppState,
    },
    prisma::{project, ModelType},
    utils::influx,
};
use ::clippy::{search_project, stream::PartialResult, Payload};

#[derive(Debug, serde::Serialize)]
pub struct PartialProject {
    id: String,
    copy: Value,
    name: String,
    model_type: ModelType,
    image_url: Option<String>,
}

impl From<project::Data> for PartialProject {
    fn from(project: project::Data) -> Self {
        Self {
            id: project.id,
            copy: project.copy,
            name: project.name,
            image_url: project.image_url,
            model_type: project.model_type,
        }
    }
}

pub async fn show(
    State(state): State<AppState>,
    ProjectFromOrigin(project): ProjectFromOrigin,
) -> ApiResult<Json<PartialProject>> {
    influx::track_load(&state.influx, &project.id)
        .await
        .map_err(|_| ApiError::ServerError("Failed to track load.".to_string()))?;

    Ok(Json(project.into()))
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct AskRequest {
    query: String,
}

pub async fn search(
    State(state): State<AppState>,
    ProjectFromOrigin(project): ProjectFromOrigin,
    Json(AskRequest { query }): Json<AskRequest>,
) -> ApiResult<Json<Vec<Payload>>> {
    influx::track_search(&state.influx, &project.id)
        .await
        .map_err(|_| ApiError::ServerError("Failed to track load.".to_string()))?;

    let results = search_project(
        &project
            .index_name
            .expect("Trained models should have an index set."),
        &query,
    )
    .await
    .map_err(|_| ApiError::ServerError("Failed to search project.".to_string()))?;

    Ok(Json(
        results.into_iter().map(|r| r.payload).collect::<Vec<_>>(),
    ))
}

#[derive(Debug, serde::Serialize)]
pub struct StreamError {
    pub error: &'static str,
}

#[allow(clippy::unused_async)]
pub async fn stream(
    State(state): State<AppState>,
    ProjectFromOrigin(project): ProjectFromOrigin,
    Json(AskRequest { query }): Json<AskRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    influx::track_query(&state.influx, &project.id)
        .await
        .unwrap();

    let stream = clippy::stream::ask(
        project
            .index_name
            .expect("Trained models should have an index set."),
        query,
        project.model_type.into(),
    );

    let stream = stream.map(|e| {
        let Ok(event) = e else {
            return Ok::<_, Infallible>(Event::default().id("error").json_data(StreamError {
                error: "Failed to complete query."
            }).unwrap())
        };

        match event {
            PartialResult::References(results) => Ok::<_, Infallible>(
                Event::default()
                    .id("references")
                    .json_data(results)
                    .unwrap(),
            ),
            PartialResult::PartialAnswer(answer) => {
                Ok::<_, Infallible>(Event::default().id("partial_answer").data(answer))
            }
            PartialResult::Error(_) => Ok::<_, Infallible>(
                Event::default()
                    .id("error")
                    .json_data(StreamError {
                        error: "Something went wrong!",
                    })
                    .unwrap(),
            ),
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
