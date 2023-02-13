use axum::Router;
use std::env;
use tower_http::{
    cors::{AllowCredentials, AllowHeaders, AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    axum::{session, state},
    routers,
    utils::db,
};

const REQUIRED_ENV_VARS: &[&str] = &[
    "APP_KEY",
    "APP_URL",
    "MAIL_FROM",
    "QDRANT_URL",
    "DATABASE_URL",
    "OPENAI_API_KEY",
    "POSTMARK_TOKEN",
];

pub async fn create() -> Router {
    for var in REQUIRED_ENV_VARS {
        assert!(env::var(var).is_ok(), "${var} not set");
    }

    let prisma = db::new().await.unwrap();
    db::migrate(&prisma)
        .await
        .expect("Failed to migrate database");

    Router::new()
        .merge(routers::mount())
        .layer(session::layer())
        .layer(
            CorsLayer::permissive()
                .allow_origin(AllowOrigin::mirror_request())
                .allow_headers(AllowHeaders::mirror_request())
                .allow_credentials(AllowCredentials::predicate(|origin, _| {
                    let origin = origin.to_str().unwrap_or("");

                    origin.ends_with("clippy.help") || origin == "http://localhost:3000"
                })),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state::create(prisma))
}
