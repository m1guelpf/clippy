use indoc::formatdoc;
use serde_json::json;

use crate::qdrant::PointResult;

#[must_use]
pub fn build_prompt(query: &str, sources: &[PointResult]) -> String {
    formatdoc!(
        "Given some extracts of a project's documentation and a question, return a JSON object, with:
        - an \"answer\" key, with a helpful answer to the provided question
        - a \"sources\" key containing at most 2 paths of the most relevant provided sources.

        If you don't know the answer, answer that you don't know and provide an empty sources array. Don't try to make up an answer and don't answer questions unrelated to the project.
    INPUT: {}
    OUTPUT:",
        json!({
            "question": query,
            "extracts": sources
        })
    )
}
