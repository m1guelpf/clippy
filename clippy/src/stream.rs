use async_fn_stream::try_fn_stream;
use async_openai::{error::OpenAIError, types::CreateChatResponse};
use futures::{Stream, StreamExt};

use crate::{
    build_messages,
    openai::ModelType,
    qdrant::{Payload, PointResult},
    OpenAI, Qdrant,
};

#[derive(Debug)]
pub enum PartialResult {
    Error(String),
    PartialAnswer(String),
    References(Vec<Payload>),
}

impl From<Result<CreateChatResponse, OpenAIError>> for PartialResult {
    fn from(answer: Result<CreateChatResponse, OpenAIError>) -> Self {
        match answer {
            Ok(res) => {
                Self::PartialAnswer(res.choices.into_iter().map(|c| c.message.content).collect())
            }
            Err(e) => Self::Error(e.to_string()),
        }
    }
}

impl From<&Vec<PointResult>> for PartialResult {
    fn from(results: &Vec<PointResult>) -> Self {
        Self::References(results.iter().map(|p| p.payload.clone()).collect())
    }
}

pub fn ask(
    project_id: String,
    query: String,
    model_type: ModelType,
) -> impl Stream<Item = std::result::Result<PartialResult, anyhow::Error>> {
    try_fn_stream(|emitter| async move {
        let client = OpenAI::new();
        let query_points = client.raw_embed(&query).await?;

        let qdrant = Qdrant::new().collection(&project_id);
        let results = qdrant.query(query_points).await?;
        emitter.emit((&results).into()).await;

        let mut answer_stream = client
            .chat_stream(
                build_messages(&query, &results.iter().map(Into::into).collect::<Vec<_>>()),
                model_type,
            )
            .await?;

        while let Some(response) = answer_stream.next().await {
            emitter.emit(response.into()).await;
        }

        Ok(())
    })
}
