use indoc::formatdoc;
use serde_json::json;

use crate::qdrant::PointResult;

#[must_use]
pub fn build_prompt(query: &str, sources: &[PointResult]) -> String {
    formatdoc!(
        "Given the following extracted parts of a project's documentation and a question, create a final helpful answer with references (\"sources\") to the 2 most relevant sources.
        If you don't know the answer, just answer that you don't know and provide an empty sources array. Don't try to make up an answer.
        ALWAYS return a \"sources\" JSON key in your answer with the most relevant paths.

    INPUT: {}
    OUTPUT:",
        json!({
            "query": query,
            "sources": sources
        })
    )
}
