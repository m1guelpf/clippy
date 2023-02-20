use indoc::formatdoc;
use serde_json::json;

use crate::qdrant::PointResult;

#[must_use]
pub fn build_prompt(query: &str, sources: &[PointResult]) -> String {
    formatdoc!(
        "Given the following extracted parts of a project's documentation and a question, create a concise answer with inline references as Markdown links when relevant.
        If you don't know the answer or the question is not related to the project, just say that you don't know. Don't try to make up an answer.

        QUESTION: {}
        =========
        EXTRACTS: {}
        ========
        ANSWER:",
        query,
        json!(sources)
    )
}
