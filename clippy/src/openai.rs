use anyhow::{anyhow, Result};
use async_openai::{
    types::{
        ChatCompletionRequestMessage, ChatCompletionResponseStream,
        CreateChatCompletionRequestArgs, CreateEmbeddingRequestArgs,
    },
    Client,
};
use backoff::ExponentialBackoffBuilder;
use futures::future;
use std::{sync::Arc, time::Duration};
use tracing::info;
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
            .map(|s| {
                format!(
                    "{}{}",
                    s.title.as_ref().map_or(String::new(), |t| format!("{t}: ")),
                    s.content.replace('\n', " ")
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

            info!(
                text = input.content,
                "Generated embeddings for {} tokens.", response.usage.total_tokens
            );

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

        info!(
            text,
            "Generated embeddings for {} tokens.", response.usage.total_tokens
        );

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
    pub async fn chat(&self, messages: Vec<ChatCompletionRequestMessage>) -> Result<String> {
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .temperature(0.5)
            .messages(messages.clone())
            .build()?;

        let response = self.client.chat().create(request).await?;

        info!(messages = ?messages, usage = ?response.usage, "Prompted gpt-3.5-turbo model.");

        Ok(response
            .choices
            .first()
            .ok_or_else(|| anyhow!("Could not find completion"))?
            .message
            .content
            .trim()
            .to_string())
    }

    /// Prompts GPT-3 to generate an answer, returning a stream of responses.
    ///
    /// # Errors
    ///
    /// This function will panic if the Completions API returns an error.
    pub async fn chat_stream(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<ChatCompletionResponseStream> {
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .temperature(0.5)
            .messages(messages.clone())
            .build()?;

        info!(
            messages = ?messages,
            "Prompting gpt-3.5-turbo model and streaming output."
        );

        Ok(self.client.chat().create_stream(request).await?)
    }
}

impl Default for OpenAI {
    fn default() -> Self {
        Self::new()
    }
}
