use axum::{
    extract::{FromRequestParts, Path},
    http::{request, HeaderValue},
    response::IntoResponse,
    routing::{get, post},
    Json, Router, Server,
};
use dotenvy::dotenv;
use serde_json::Value;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use ::clippy::{build_prompt, OpenAI, Qdrant};

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "axum_api=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cors = CorsLayer::new().allow_origin(AllowOrigin::predicate(
        |origin: &HeaderValue, request: &request::Parts| {
            if request.uri == "/" || origin == "http://localhost:3000" {
                return true;
            }

            if !request.uri.path().ends_with("/query") {
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
        .layer(cors);

    let addr = "0.0.0.0:3000".parse().unwrap();
    tracing::info!("Listening on {}", addr);
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, serde::Deserialize)]
struct AskRequest {
    query: String,
}

async fn ask(Path(project): Path<String>, Json(req): Json<AskRequest>) -> impl IntoResponse {
    let client = OpenAI::new();
    let qdrant = Qdrant::new().collection(&format!("docs_{project}"));

    let query_points = client.raw_embed(&req.query).await.unwrap();
    let results = qdrant.query(query_points).await.unwrap();

    Json::<Value>(
        serde_json::to_value(
            client
                .prompt(&build_prompt(&req.query, &results))
                .await
                .unwrap(),
        )
        .unwrap(),
    )
}
