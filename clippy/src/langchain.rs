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
                "You are a very enthusiastic company representative who loves to help people! Given the following sections from the documentation, give a comprehensive answer to the user's question, providing inline references in `[page title](path)` format (when relevant).
                If you are unsure or the question doesn't relate to the project, say \"Sorry, I am not sure how to answer that.\"

                Documentation:
                ---
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
