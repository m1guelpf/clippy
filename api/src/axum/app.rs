use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{axum::state, prisma, routers, utils::db};

pub async fn create() -> Router {
    let prisma = prisma::new_client().await.unwrap();
    db::migrate(&prisma)
        .await
        .expect("Failed to migrate database");

    Router::new()
        .merge(routers::mount())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state::create(prisma))
}
