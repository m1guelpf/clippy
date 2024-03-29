use anyhow::Result;
use tracing::info;

#[cfg(not(debug_assertions))]
use std::time::Duration;
#[cfg(not(debug_assertions))]
use tokio::time::sleep;

use crate::prisma::{self, PrismaClient};

async fn _migrate(client: &PrismaClient) -> Result<()> {
    #[cfg(debug_assertions)]
    client._db_push().await?;

    #[cfg(not(debug_assertions))]
    client._migrate_deploy().await?;

    Ok(())
}

pub async fn new() -> Result<PrismaClient> {
    // Wait for database to be ready in production
    #[cfg(not(debug_assertions))]
    sleep(Duration::from_secs(1)).await;

    Ok(prisma::new_client().await?)
}

pub async fn migrate(client: &PrismaClient) -> Result<()> {
    _migrate(client).await?;

    info!("Database migrated");

    Ok(())
}
