use async_fn_stream::try_fn_stream;
use futures::Stream;

use crate::{
    build_prompt,
    openai::Answer,
    qdrant::{Payload, PointResult},
    OpenAI, Qdrant,
};

pub enum PartialResult {
    Answer(Answer),
    References(Vec<Payload>),
}

impl From<Answer> for PartialResult {
    fn from(answer: Answer) -> Self {
        Self::Answer(answer)
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
) -> impl Stream<Item = std::result::Result<PartialResult, anyhow::Error>> {
    try_fn_stream(|emitter| async move {
        let client = OpenAI::new();
        let query_points = client.raw_embed(&query).await?;

        let qdrant = Qdrant::new().collection(&format!("docs_{project_id}"));
        let results = qdrant.query(query_points).await?;
        emitter.emit((&results).into()).await;

        let answer = client.prompt(&build_prompt(&query, &results)).await?;
        emitter.emit(answer.into()).await;

        Ok(())
    })
}
