use std::convert::Infallible;

use axum::response::{
    sse::{Event, KeepAlive},
    Sse,
};
use axum_jsonschema::Json;
use futures::Stream;
use map_macro::map;
use schemars::JsonSchema;
use serde_json::Value;
use tokio_stream::StreamExt;

use crate::{
    axum::{errors::ApiResult, extractors::ProjectFromOrigin},
    prisma::project,
};
use ::clippy::{search_project, stream::PartialResult};

#[derive(Debug, serde::Serialize)]
pub struct PartialProject {
    id: String,
    copy: Value,
    image_url: Option<String>,
}

impl From<project::Data> for PartialProject {
    fn from(project: project::Data) -> Self {
        Self {
            id: project.id,
            copy: project.copy,
            image_url: project.image_url,
        }
    }
}

#[allow(clippy::unused_async)]
pub async fn show(ProjectFromOrigin(project): ProjectFromOrigin) -> Json<PartialProject> {
    Json(project.into())
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct AskRequest(#[serde(rename = "query")] String);

pub async fn search(
    ProjectFromOrigin(project): ProjectFromOrigin,
    Json(AskRequest(query)): Json<AskRequest>,
) -> ApiResult<Json<Value>> {
    let results = search_project(
        &project
            .index_name
            .expect("Trained models should have an index set."),
        &query,
    )
    .await
    .unwrap();

    Ok(Json(
        serde_json::to_value(
            results
                .into_iter()
                .map(|r| {
                    map! {
                        "path" => r.payload.path,
                        "text" => r.payload.text,
                        "title" => r.payload.title,
                        "page" => r.payload.page_title,
                    }
                })
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    ))
}

#[derive(Debug, serde::Serialize)]
struct StreamError {
    error: &'static str,
}

#[allow(clippy::unused_async)]
pub async fn stream(
    ProjectFromOrigin(project): ProjectFromOrigin,
    Json(AskRequest(query)): Json<AskRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = clippy::stream::ask(
        project
            .index_name
            .expect("Trained models should have an index set."),
        query,
        project.model_type.into(),
    );

    let stream = stream.map(|e| {
        let Ok(event) = e else {
            return Ok::<_, Infallible>(Event::default().id("error").json_data(StreamError{
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
            PartialResult::Answer(answer) => {
                Ok::<_, Infallible>(Event::default().id("answer").json_data(answer).unwrap())
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
