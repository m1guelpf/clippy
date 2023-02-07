#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use axum::{
    async_trait,
    extract::{FromRequestParts, Path, State},
    http::{request::Parts, HeaderMap, StatusCode},
    response::IntoResponse,
    response::{
        sse::{Event, KeepAlive, Sse},
        Result,
    },
    routing::{get, post},
    Json, RequestPartsExt, Router, Server,
};
use axum_derive_error::ErrorResponse;
use dotenvy::dotenv;
use futures::stream::Stream;
use map_macro::map;
use pika::pika::{InitOptions, Pika, PrefixRecord};
use prisma::{project, PrismaClient};
use serde_json::{json, Value};
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use thiserror::Error;
use tokio_stream::StreamExt as _;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use url::Url;

mod prisma;
mod utils;

use crate::utils::db_migrate;
use ::clippy::{build_prompt, search_project, stream::PartialResult, OpenAI};

#[derive(Debug)]
struct AppState {
    pika: Pika,
    prisma: PrismaClient,
}

#[derive(Error, ErrorResponse)]
pub enum AppError {
    #[error("Project not found.")]
    #[status(StatusCode::NOT_FOUND)]
    ProjectNotFound,

    #[error("{0}")]
    #[status(StatusCode::BAD_REQUEST)]
    ClientError(String),
}

struct Project(project::Data);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for Project {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let State(state) = parts
            .extract_with_state::<State<Arc<AppState>>, Arc<AppState>>(state)
            .await
            .unwrap();

        let Path(path) = parts
            .extract::<Path<HashMap<String, String>>>()
            .await
            .unwrap();

        let path = path
            .get("project")
            .ok_or_else(|| AppError::ClientError("Missing project ID".to_string()))?
            .clone();

        let project = state
            .prisma
            .project()
            .find_unique(project::UniqueWhereParam::IdEquals(path))
            .exec()
            .await;

        match project {
            Ok(Some(project)) => Ok(Self(project)),
            _ => Err(AppError::ProjectNotFound),
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "clippy=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    #[cfg(not(debug_assertions))]
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let prefixes = vec![
        PrefixRecord {
            prefix: "user".to_string(),
            description: Some("User ID".to_string()),
            secure: false,
        },
        PrefixRecord {
            prefix: "team".to_string(),
            description: Some("Team ID".to_string()),
            secure: false,
        },
        PrefixRecord {
            prefix: "proj".to_string(),
            description: Some("Project ID".to_string()),
            secure: false,
        },
    ];
    let pika = Pika::new(prefixes, &InitOptions::default());

    let prisma = prisma::new_client().await.unwrap();
    db_migrate(&prisma)
        .await
        .expect("Failed to migrate database");

    let shared_state = Arc::new(AppState { pika, prisma });

    let app = Router::new()
        .route("/", get(|| async {}))
        .route("/widget", get(widget_info))
        .route("/:project", get(project_info))
        .route("/:project/query", post(ask))
        .route("/:project/search", post(search))
        .route("/:project/stream", post(stream))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(shared_state);

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

async fn widget_info(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let origin = headers
        .get("origin")
        .ok_or_else(|| AppError::ClientError("Invalid request".to_string()))?
        .to_str()
        .unwrap();

    let origin = Url::parse(origin).unwrap().host().unwrap().to_string();

    dbg!(&origin);

    let project = state
        .prisma
        .project()
        .find_first(vec![project::WhereParam::Origins(
            prisma::read_filters::JsonFilter::ArrayContains(Some(origin.into())),
        )])
        .select(project::select! ({ id image_url copy }))
        .exec()
        .await;

    let Ok(Some(project)) = project else {
        return Err(AppError::ProjectNotFound);
    };

    Ok(Json(serde_json::to_value(project).unwrap()))
}

#[allow(clippy::unused_async)]
async fn project_info(Project(project): Project) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::to_value(project).unwrap()))
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
