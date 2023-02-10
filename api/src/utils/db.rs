use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;
use tracing::info;

use crate::prisma::PrismaClient;

async fn _migrate(client: &PrismaClient) -> Result<()> {
    #[cfg(debug_assertions)]
    client._db_push().await?;

    #[cfg(not(debug_assertions))]
    client._migrate_deploy().await?;

    Ok(())
}

pub async fn migrate(client: &PrismaClient) -> Result<()> {
    _migrate(client).await?;

    info!("Database migrated");

    Ok(())
}
