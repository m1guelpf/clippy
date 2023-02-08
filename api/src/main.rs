#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use ::axum::Server;
use dotenvy::dotenv;
use std::net::SocketAddr;

use crate::{axum::app, utils::logger};

mod axum;
mod prisma;
mod routers;
mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();
    logger::setup();

    #[cfg(not(debug_assertions))]
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let app = app::create().await;
    let address = SocketAddr::from(([0, 0, 0, 0], 3000));

    println!("Server listening on {address}");
    Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .expect("Failed to start server");
}
