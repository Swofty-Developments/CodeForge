//! Message types for AI conversations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A message in an AI conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", content = "content")]
pub enum Message {
    /// A message from the human user.
    User(Vec<ContentBlock>),
    /// A response from the AI assistant.
    Assistant(Vec<ContentBlock>),
    /// A system-level instruction.
    System(String),
    /// A tool use request from the assistant.
    ToolUse {
        /// Unique ID for this tool use invocation.
        tool_use_id: String,
        /// The name of the tool being invoked.
        tool_name: String,
        /// The input parameters as a JSON value.
        input: serde_json::Value,
    },
    /// The result of a tool execution.
    ToolResult {
        /// The tool use ID this result corresponds to.
        tool_use_id: String,
        /// The output content blocks.
        content: Vec<ContentBlock>,
        /// Whether the tool execution encountered an error.
        is_error: bool,
    },
}

impl Message {
    /// Create a simple user text message.
    pub fn user(text: impl Into<String>) -> Self {
        Self::User(vec![ContentBlock::Text(text.into())])
    }

    /// Create a simple assistant text message.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::Assistant(vec![ContentBlock::Text(text.into())])
    }

    /// Create a system message.
    pub fn system(text: impl Into<String>) -> Self {
        Self::System(text.into())
    }

    /// Returns the role of this message as a string.
    pub fn role(&self) -> &'static str {
        match self {
            Self::User(_) => "user",
            Self::Assistant(_) => "assistant",
            Self::System(_) => "system",
            Self::ToolUse { .. } => "tool_use",
            Self::ToolResult { .. } => "tool_result",
        }
    }

    /// Extract all text content from this message, concatenated.
    pub fn text_content(&self) -> String {
        match self {
            Self::User(blocks) | Self::Assistant(blocks) | Self::ToolResult { content: blocks, .. } => {
                blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Self::System(text) => text.clone(),
            Self::ToolUse { .. } => String::new(),
        }
    }
}

/// A content block within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Plain text content.
    Text(String),
    /// Thinking / chain-of-thought content (extended thinking).
    Thinking {
        /// The thinking text.
        text: String,
        /// Signature for verifying thinking block authenticity.
        signature: Option<String>,
    },
    /// An image provided as base64 data.
    Image {
        /// The media type (e.g., `image/png`).
        media_type: String,
        /// Base64-encoded image data.
        data: String,
    },
    /// A tool use invocation block.
    ToolUse {
        /// The tool use ID.
        id: String,
        /// Tool name.
        name: String,
        /// Tool input as JSON.
        input: serde_json::Value,
    },
}

/// A timestamped message with metadata for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    /// Unique message identifier.
    pub id: Uuid,
    /// The thread this message belongs to.
    pub thread_id: Uuid,
    /// The message content.
    pub message: Message,
    /// When this message was created.
    pub created_at: DateTime<Utc>,
    /// Token count for this message, if known.
    pub token_count: Option<u64>,
}

impl StoredMessage {
    /// Create a new stored message with the current timestamp.
    pub fn new(thread_id: Uuid, message: Message) -> Self {
        Self {
            id: Uuid::new_v4(),
            thread_id,
            message,
            created_at: Utc::now(),
            token_count: None,
        }
    }

    /// Set the token count for this message.
    pub fn with_tokens(mut self, count: u64) -> Self {
        self.token_count = Some(count);
        self
    }
}
