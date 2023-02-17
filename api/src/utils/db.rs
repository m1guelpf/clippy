use anyhow::Result;
use schemars::JsonSchema;
use tracing::info;

#[cfg(not(debug_assertions))]
use std::time::Duration;
#[cfg(not(debug_assertions))]
use tokio::time::sleep;

use crate::prisma::{self, ModelType, PrismaClient};
use ::clippy::openai;

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

#[derive(serde::Deserialize, JsonSchema)]
#[serde(remote = "prisma::ModelType")]
pub enum ModelTypeDef {
    #[serde(rename = "Metal")]
    Metal,
    #[serde(rename = "Plastic")]
    Plastic,
}

impl From<ModelType> for openai::ModelType {
    fn from(val: ModelType) -> Self {
        match val {
            ModelType::Metal => Self::Davinci,
            ModelType::Plastic => Self::Curie,
        }
    }
}
