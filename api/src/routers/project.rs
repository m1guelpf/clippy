use std::convert::Infallible;

use axum::{
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::Stream;
use map_macro::map;
use serde_json::{json, Value};
use tokio_stream::StreamExt;

use crate::axum::{extractors::Project, state::AppState};
use ::clippy::{build_prompt, search_project, stream::PartialResult, OpenAI};

pub fn mount() -> Router<AppState> {
    Router::new().nest(
        "/:project",
        Router::new()
            .route("/", get(project_info))
            .route("/search", post(search))
            .route("/ask", post(ask))
            .route("/stream", post(stream)),
    )
}

#[allow(clippy::unused_async)]
async fn project_info(Project(project): Project) -> impl IntoResponse {
    Json(project)
}

#[derive(Debug, serde::Deserialize)]
struct AskRequest {
    query: String,
}

async fn search(Project(project): Project, Json(req): Json<AskRequest>) -> impl IntoResponse {
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

async fn ask(Project(project): Project, Json(req): Json<AskRequest>) -> impl IntoResponse {
    let client = OpenAI::new();
    let results = search_project(&project.index_name, &req.query)
        .await
        .unwrap();

    let answer = client
        .prompt(&build_prompt(&req.query, &results))
        .await
        .unwrap();

    let mut results = results.into_iter();

    Json(json!({
        "answer": answer.answer,
        "sources": answer
        .sources
        .into_iter()
        .map(|path| {
            results
                .find(|r| r.payload.path == path)
                .map(|r| map! {
                    "path" => r.payload.path,
                    "title" => r.payload.title,
                    "page" => r.payload.page_title,
                })
        })
        .collect::<Vec<_>>(),
    }))
}

#[allow(clippy::unused_async)]
async fn stream(
    Project(project): Project,
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
