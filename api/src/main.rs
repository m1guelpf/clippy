#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use axum::{
    extract::Path,
    http::{request, HeaderValue},
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
    routing::{get, post},
    Json, Router, Server,
};
use dotenvy::dotenv;
use futures::stream::Stream;
use map_macro::map;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::StreamExt as _;
use tower_http::cors::{AllowOrigin, CorsLayer};

use ::clippy::{build_prompt, search_project, stream::PartialResult, OpenAI};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let cors = CorsLayer::permissive().allow_origin(AllowOrigin::predicate(
        |origin: &HeaderValue, request: &request::Parts| {
            if request.uri == "/" || origin == "http://localhost:3000" {
                return true;
            }

            if !request.uri.path().ends_with("/query") && !request.uri.path().ends_with("/search") {
                return false;
            }

            let project = request.uri.path().split('/').nth(1).unwrap();

            if project == "hop" && origin == "https://docs.hop.io" {
                return true;
            }

            false
        },
    ));

    let app = Router::new()
        .route("/", get(|| async {}))
        .route("/:project/query", post(ask))
        .route("/:project/search", post(search))
        .route("/:project/stream", post(stream))
        .layer(cors);

    let addr = "0.0.0.0:3000".parse().unwrap();

    println!("Listening on {addr}");
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, serde::Deserialize)]
struct AskRequest {
    query: String,
}

async fn search(Path(project): Path<String>, Json(req): Json<AskRequest>) -> impl IntoResponse {
    let results = search_project(&project, &req.query).await.unwrap();

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

async fn ask(Path(project): Path<String>, Json(req): Json<AskRequest>) -> impl IntoResponse {
    let client = OpenAI::new();
    let results = search_project(&project, &req.query).await.unwrap();

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
    Path(project): Path<String>,
    Json(req): Json<AskRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = clippy::stream::ask(project, req.query);

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
