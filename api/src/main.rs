#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use axum::{
    extract::Path,
    http::{request, HeaderValue},
    response::IntoResponse,
    routing::{get, post},
    Json, Router, Server,
};
use dotenvy::dotenv;
use map_macro::map;
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};

use ::clippy::{build_prompt, search_project, OpenAI};

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

            if project == "hop" && origin == "https://clippy-widget.vercel.app" {
                return true;
            }

            false
        },
    ));

    let app = Router::new()
        .route("/", get(|| async {}))
        .route("/:project/query", post(ask))
        .route("/:project/search", post(search))
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
                .unwrap()
        })
        .collect::<Vec<_>>(),
    }))
}
