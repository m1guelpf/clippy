use axum::{
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    routing::{get, post},
    Router,
};
use axum_jsonschema::Json;
use futures::Stream;
use map_macro::map;
use schemars::JsonSchema;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::StreamExt;

use crate::{
    axum::{extractors::ProjectFromOrigin, state::AppState},
    prisma::project,
};
use ::clippy::{search_project, stream::PartialResult};

pub fn mount() -> Router<AppState> {
    Router::new().nest(
        "/widget",
        Router::new()
            .route("/", get(widget_info))
            .route("/search", get(search))
            .route("/stream", post(stream)),
    )
}

#[derive(Debug, serde::Serialize)]
struct WidgetInfoResponse {
    id: String,
    copy: Value,
    image_url: Option<String>,
}

impl From<project::Data> for WidgetInfoResponse {
    fn from(project: project::Data) -> Self {
        Self {
            id: project.id,
            copy: project.copy,
            image_url: project.image_url,
        }
    }
}

#[allow(clippy::unused_async)]
async fn widget_info(ProjectFromOrigin(project): ProjectFromOrigin) -> Json<WidgetInfoResponse> {
    Json(project.into())
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
struct AskRequest {
    query: String,
}

async fn search(
    ProjectFromOrigin(project): ProjectFromOrigin,
    Json(req): Json<AskRequest>,
) -> Json<Value> {
    let results = search_project(&project.index_name, &req.query)
        .await
        .unwrap();

    Json(
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
    )
}

#[allow(clippy::unused_async)]
async fn stream(
    ProjectFromOrigin(project): ProjectFromOrigin,
    Json(req): Json<AskRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = clippy::stream::ask(project.index_name, req.query, project.model_type.into());

    let stream = stream.map(|e| {
        let Ok(event) = e else {
            return Ok::<_, Infallible>(Event::default().id("error").json_data(json!({
                "error": "Failed to complete query."
            })).unwrap())
        };

        match event {
            PartialResult::References(results) => {
                let results = results
                    .into_iter()
                    .map(|p| {
                        json!({
                            "path": p.path,
                            "text": p.text,
                            "title": p.title,
                            "page": p.page_title,
                        })
                    })
                    .collect::<Vec<Value>>();

                Ok::<_, Infallible>(
                    Event::default()
                        .id("references")
                        .json_data(results)
                        .unwrap(),
                )
            }
            PartialResult::Answer(answer) => {
                Ok::<_, Infallible>(Event::default().id("answer").json_data(answer).unwrap())
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
