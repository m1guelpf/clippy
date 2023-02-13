use axum::{
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::Stream;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::StreamExt;

use crate::{
    axum::{extractors::ProjectFromOrigin, state::AppState},
    prisma::project,
};
use ::clippy::stream::PartialResult;

pub fn mount() -> Router<AppState> {
    Router::new().nest(
        "/widget",
        Router::new()
            .route("/", get(widget_info))
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

#[derive(Debug, serde::Deserialize)]
struct AskRequest {
    query: String,
}

#[allow(clippy::unused_async)]
async fn stream(
    ProjectFromOrigin(project): ProjectFromOrigin,
    Json(req): Json<AskRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = clippy::stream::ask(project.index_name, req.query);

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
