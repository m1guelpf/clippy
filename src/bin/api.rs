use axum::{
    response::IntoResponse,
    routing::{get, post},
    Json, Router, Server,
};
use dotenvy::dotenv;
use serde_json::Value;

use ::clippy::{build_prompt, OpenAI, Qdrant};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app = Router::new()
        .route("/", get(|| async {}))
        .route("/ask", post(ask));

    Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, serde::Deserialize)]
struct AskRequest {
    query: String,
}

async fn ask(Json(req): Json<AskRequest>) -> impl IntoResponse {
    let client = OpenAI::new();
    let qdrant = Qdrant::new().collection("docs_hop");

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
