use anyhow::{anyhow, Result};
use async_openai::{
    types::{CompletionResponseStream, CreateCompletionRequestArgs, CreateEmbeddingRequestArgs},
    Client,
};
use backoff::ExponentialBackoffBuilder;
use futures::future;
use std::{
    fmt::{self, Display},
    sync::Arc,
    time::Duration,
};
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

#[derive(Debug)]
pub enum ModelType {
    Davinci,
    Curie,
}

impl From<ModelType> for String {
    fn from(model_type: ModelType) -> Self {
        model_type.to_string()
    }
}

impl Display for ModelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Davinci => write!(f, "text-davinci-003"),
            Self::Curie => write!(f, "text-curie-001"),
        }
    }
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
            .map(|s| {
                format!(
                    "{}{}",
                    s.title.as_ref().map_or(String::new(), |t| format!("{t}: ")),
                    s.content
                )
            })
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
                    .ok_or_else(|| anyhow!("Could not find embedding"))?
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
            .ok_or_else(|| anyhow!("Could not find embedding"))?
            .embedding
            .clone())
    }

    /// Prompts GPT-3 to generate an answer.
    ///
    /// # Errors
    ///
    /// This function will panic if the Completions API returns an error.
    pub async fn prompt(&self, prompt: &str, model_type: ModelType) -> Result<String> {
        let request = CreateCompletionRequestArgs::default()
            .max_tokens(400_u16)
            .model(model_type)
            .temperature(0.5)
            .prompt(prompt)
            .build()?;

        let response = self.client.completions().create(request).await?;

        Ok(response
            .choices
            .first()
            .ok_or_else(|| anyhow!("Could not find completion"))?
            .text
            .trim()
            .to_string())
    }

    /// Prompts GPT-3 to generate an answer, returning a stream of responses.
    ///
    /// # Errors
    ///
    /// This function will panic if the Completions API returns an error.
    pub async fn prompt_stream(
        &self,
        prompt: &str,
        model_type: ModelType,
    ) -> Result<CompletionResponseStream> {
        let request = CreateCompletionRequestArgs::default()
            .max_tokens(400_u16)
            .model(model_type)
            .temperature(0.5)
            .prompt(prompt)
            .build()?;

        Ok(self.client.completions().create_stream(request).await?)
    }
}

impl Default for OpenAI {
    fn default() -> Self {
        Self::new()
    }
}
