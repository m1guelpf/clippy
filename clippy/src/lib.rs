#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod langchain;
pub mod openai;
mod parser;
mod qdrant;
pub mod stream;

pub use langchain::{build_prompt, Context};
pub use openai::OpenAI;
pub use parser::{into_document, Document};
pub use qdrant::{Payload, Qdrant};

use anyhow::Result;
use qdrant::PointResult;

/// Searches a project's documentation.
///
/// # Errors
///
/// This function will panic if the `Qdrant` or the `OpenAI` APIs return an error.
pub async fn search_project(project_id: &str, query: &str) -> Result<Vec<PointResult>> {
    let client = OpenAI::new();
    let qdrant = Qdrant::new().collection(project_id);

    let query_points = client.raw_embed(query).await?;
    qdrant.query(query_points).await
}
