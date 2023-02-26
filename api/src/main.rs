#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use ::axum::Server;
use dotenvy::dotenv;
use std::{env, net::SocketAddr};
use tracing::info;

use crate::{axum::app, utils::logger};

mod axum;
mod http;
mod prisma;
mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let _guard = logger::setup();

    let app = app::create().await;
    let address = SocketAddr::from((
        [0, 0, 0, 0],
        env::var("PORT").map_or(8000, |p| p.parse().unwrap()),
    ));

    info!("⚡ Clippy API started on http://{address}");
    Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .expect("Failed to start server");
}
