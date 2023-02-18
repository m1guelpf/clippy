#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use ::axum::Server;
use dotenvy::dotenv;
use std::net::SocketAddr;

use crate::{axum::app, utils::logger};

mod axum;
mod http;
mod prisma;
mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();
    logger::setup();

    let app = app::create().await;
    let address = SocketAddr::from(([0, 0, 0, 0], 8000));

    println!("⚡ Clippy API started on http://{address}");
    Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .expect("Failed to start server");
}
