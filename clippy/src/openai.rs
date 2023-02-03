use anyhow::Result;
use async_openai::{
    types::{CreateCompletionRequestArgs, CreateEmbeddingRequestArgs},
    Client,
};
use backoff::ExponentialBackoffBuilder;
use futures::future;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

use crate::{
    parser::Document,
    qdrant::{Payload, PointStruct},
};

pub struct OpenAI {
    client: Arc<Client>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Answer {
    pub answer: String,
    pub sources: Vec<String>,
}

impl OpenAI {
    #[must_use]
    pub fn new() -> Self {
        let backoff = ExponentialBackoffBuilder::new()
            .with_max_elapsed_time(Some(Duration::from_secs(60)))
            .build();

        Self {
            client: Arc::new(Client::new().with_backoff(backoff)),
        }
    }

    /// Embeds a document into a vector of points.
    ///
    /// # Errors
    ///
    /// This function will panic if the Embeddings API returns an error.
    pub async fn embed(&self, document: &Document) -> Result<Vec<PointStruct>> {
        let tokens = document
            .sections
            .iter()
            .map(|s| s.content.clone())
            .collect::<Vec<String>>();

        let mut responses = Vec::new();

        for token in tokens {
            let client = self.client.clone();
            let request = CreateEmbeddingRequestArgs::default()
                .model("text-embedding-ada-002")
                .input(token)
                .build()?;

            responses.push(tokio::spawn(async move {
                client.embeddings().create(request).await
            }));
        }

        let responses = future::join_all(responses).await;

        let mut points = Vec::new();

        for (i, response) in responses.into_iter().enumerate() {
            let input = &document.sections[i];
            let response = response??;

            let point = PointStruct {
                id: Uuid::new_v4().to_string(),
                payload: Payload {
                    text: input.content.clone(),
                    path: document.path.clone(),
                    page_title: document.title.clone(),
                    title: input.title.clone().unwrap_or_default(),
                },
                vector: response
                    .data
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("Could not find embedding"))?
                    .embedding
                    .clone(),
            };

            points.push(point);
        }

        Ok(points)
    }

    /// Embeds a string into a vector of points.
    ///
    /// # Errors
    ///
    /// This function will panic if the Embeddings API returns an error.
    pub async fn raw_embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-ada-002")
            .input(text)
            .build()?;

        let response = self.client.embeddings().create(request).await?;

        Ok(response
            .data
            .first()
            .ok_or_else(|| anyhow::anyhow!("Could not find embedding"))?
            .embedding
            .clone())
    }

    /// Prompts GPT-3 to generate an answer.
    ///
    /// # Errors
    ///
    /// This function will panic if the Completions API returns an error.
    pub async fn prompt(&self, prompt: &str) -> Result<Answer> {
        let request = CreateCompletionRequestArgs::default()
            .model("text-davinci-003")
            .temperature(0.8)
            .max_tokens(700_u16)
            .prompt(prompt)
            .build()?;

        let response = self.client.completions().create(request).await?;
        let response = response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("Could not find completion"))?
            .text
            .clone();

        serde_json::from_str(&response).map_err(Into::into)
    }
}

impl Default for OpenAI {
    fn default() -> Self {
        Self::new()
    }
}
