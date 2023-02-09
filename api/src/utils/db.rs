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
    let mut attempts = 0;

    loop {
        attempts += 1;
        let result = _migrate(client).await;

        if result.is_ok() {
            break;
        }

        if attempts > 10 {
            return Err(result.unwrap_err());
        }

        sleep(Duration::from_millis(500)).await;
    }

    info!("Database migrated");

    Ok(())
}
