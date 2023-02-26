use crate::qdrant::PointResult;
use indoc::formatdoc;
use std::fmt::Display;

pub struct Context {
    path: String,
    content: String,
}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Path: {}\nContent:{}", self.path, self.content)
    }
}

impl From<&PointResult> for Context {
    fn from(point: &PointResult) -> Self {
        Self {
            path: point.payload.path.clone(),
            content: point.payload.text.clone(),
        }
    }
}

#[must_use]
pub fn build_prompt(query: &str, sources: &[Context]) -> String {
    formatdoc!(
        "Use some of the following pieces of content to create a concise answer with inline markdown references.
        If you don't know the answer, just say that you don't know. Don't try to make up an answer.

        QUESTION: {}
        =========
        {}
        ========
        FINAL ANSWER:",
        query,
        sources.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
    )
}
