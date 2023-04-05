use std::env;

use anyhow::Result;
use futures::future::join_all;
use reqwest::Client;
use serde_json::Value;
use tracing::debug;

const EMBEDDING_SIZE: i32 = 1536;

pub struct Qdrant {
    client: Client,
    base_url: String,
}

impl Qdrant {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: env::var("QDRANT_URL").expect("$QDRANT_URL not set"),
        }
    }

    /// Creates a new Qdrant collection.
    ///
    /// # Errors
    ///
    /// This function will panic if the Qdrant API returns an error.
    pub async fn create_collection(&self, name: &str) -> Result<()> {
        self.client
            .put(&format!("{}/collections/{name}", self.base_url))
            .json(&serde_json::json!({
                "name": name,
                "vectors": {
                    "distance": "Cosine",
                    "size": EMBEDDING_SIZE,
                }
            }))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    #[must_use]
    pub fn collection(self, name: &str) -> Collection {
        Collection::new(self.client, format!("{}/collections/{name}", self.base_url))
    }
}

impl Default for Qdrant {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, opg::OpgModel)]
pub struct Payload {
    pub text: String,
    pub path: String,
    pub title: String,
    pub page_title: String,
}

#[derive(Debug, serde::Serialize)]
pub struct PointStruct {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: Payload,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PointResult {
    id: String,
    score: f32,
    pub payload: Payload,
}

pub struct Collection {
    client: Client,
    base_url: String,
}

impl Collection {
    pub const fn new(client: Client, url: String) -> Self {
        Self {
            client,
            base_url: url,
        }
    }

    pub async fn upsert(&self, vectors: &[PointStruct]) -> Result<()> {
        join_all(vectors.chunks(30).map(|chunk| async move {
            self.client
                .put(&format!("{}/points", self.base_url))
                .json(&serde_json::json!({ "points": chunk }))
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap();
        }))
        .await;

        debug!("Upserted {} vectors", vectors.len());

        Ok(())
    }

    pub async fn query(&self, vectors: Vec<f32>, count: usize) -> Result<Vec<PointResult>> {
        let results: Value = self
            .client
            .post(&format!("{}/points/search", self.base_url))
            .json(&serde_json::json!({
                "limit": count,
                "vector": vectors,
                "with_payload": true,
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(results
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("No result field in response"))?
            .as_array()
            .unwrap()
            .iter()
            .map(|r| serde_json::from_value::<PointResult>(r.clone()).unwrap())
            .collect())
    }
}
