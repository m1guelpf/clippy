use anyhow::Result;

use crate::prisma::PrismaClient;

pub async fn migrate(client: &PrismaClient) -> Result<()> {
    #[cfg(debug_assertions)]
    client._db_push().await?;

    #[cfg(not(debug_assertions))]
    client._migrate_deploy().await?;

    Ok(())
}
