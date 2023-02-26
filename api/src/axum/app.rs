use axum::Router;
use sentry::integrations::tower::{NewSentryLayer, SentryHttpLayer};
use std::env;
use tower_http::{
    cors::{AllowCredentials, AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    request_id::{PropagateRequestIdLayer, SetRequestIdLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

use crate::{
    axum::{
        session::{self, RequestIdMaker},
        state,
    },
    http::routes,
    utils::db,
};

const REQUIRED_ENV_VARS: &[&str] = &[
    "APP_KEY",
    "APP_URL",
    "INFLUX_DB",
    "MAIL_FROM",
    "INFLUX_ORG",
    "QDRANT_URL",
    "INFLUX_HOST",
    "INFLUX_TOKEN",
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
        .merge(routes::mount())
        .layer(session::layer())
        .layer(
            CorsLayer::permissive()
                .allow_origin(AllowOrigin::mirror_request())
                .allow_headers(AllowHeaders::mirror_request())
                .allow_methods(AllowMethods::mirror_request())
                .allow_credentials(AllowCredentials::predicate(|origin, _| {
                    let origin = origin.to_str().unwrap_or("");

                    origin.ends_with("clippy.help") || origin == "http://localhost:3000"
                })),
        )
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(DefaultOnResponse::new().include_headers(true)),
        )
        .layer(SetRequestIdLayer::x_request_id(RequestIdMaker::default()))
        .layer(SentryHttpLayer::with_transaction())
        .layer(NewSentryLayer::new_from_top())
        .with_state(state::create(prisma).await)
}
