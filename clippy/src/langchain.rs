use crate::qdrant::PointResult;
use async_openai::types::{ChatCompletionRequestMessage, Role};
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
pub fn build_messages(query: &str, sources: &[Context]) -> Vec<ChatCompletionRequestMessage> {
    vec![
        ChatCompletionRequestMessage {
            name: None,
            role: Role::System,
            content: formatdoc!(
                "You are a helpful assistant that summarizes documentation. Answer the user's queries with the context below, providing inline references *as Markdown links* when relevant.
                If you don't know the answer, just say that you don't know. Don't try to make up an answer.
                =========
                {}",
                sources.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
            )
        },
        ChatCompletionRequestMessage {
            name: None,
            role: Role::User,
            content: query.to_string()
        }
    ]
}
