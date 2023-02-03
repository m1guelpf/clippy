use indoc::formatdoc;
use serde_json::json;

use crate::qdrant::PointResult;

#[must_use]
pub fn build_prompt(query: &str, sources: &[PointResult]) -> String {
    formatdoc!(
        "Given the following extracts of a project's documentation, create a helpful answer to the provided question.
        ALWAYS return a JSON object with an \"answers\" key and a \"sources\" key containing the most relevant paths. Provide at max 2 sources.
        If you don't know the answer, just answer that you don't know and provide an empty sources array. Don't try to make up an answer and don't answer questions unrelated to the project.
    INPUT: {}
    OUTPUT:",
        json!({
            "query": query,
            "sources": sources
        })
    )
}
