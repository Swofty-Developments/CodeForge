//! Streaming event types for incremental AI responses.

use serde::{Deserialize, Serialize};

/// Events emitted during streaming AI response generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum StreamEvent {
    /// The session is ready and the model has been confirmed.
    SessionReady {
        /// The confirmed model identifier.
        model: String,
        /// The session ID.
        session_id: String,
    },

    /// An incremental text delta from the assistant.
    TextDelta {
        /// The text fragment.
        text: String,
    },

    /// An incremental thinking/reasoning delta.
    ThinkingDelta {
        /// The thinking text fragment.
        text: String,
    },

    /// The assistant is invoking a tool.
    ToolUseStart {
        /// The tool use ID.
        tool_use_id: String,
        /// The tool name.
        tool_name: String,
    },

    /// Incremental input JSON for a tool use.
    ToolInputDelta {
        /// The tool use ID.
        tool_use_id: String,
        /// The JSON fragment.
        json: String,
    },

    /// The tool use input is complete.
    ToolUseEnd {
        /// The tool use ID.
        tool_use_id: String,
    },

    /// The assistant's turn is complete.
    MessageComplete {
        /// Total input tokens for this turn.
        input_tokens: u64,
        /// Total output tokens for this turn.
        output_tokens: u64,
    },

    /// An error occurred during streaming.
    Error {
        /// Error message.
        message: String,
        /// Whether the error is recoverable.
        recoverable: bool,
    },

    /// The stream has ended (session complete or interrupted).
    Done {
        /// The reason the stream ended.
        reason: StopReason,
    },
}

/// Why the streaming response stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model finished its response naturally.
    EndTurn,
    /// The model wants to use a tool.
    ToolUse,
    /// The maximum token limit was reached.
    MaxTokens,
    /// The user interrupted the generation.
    UserInterrupt,
    /// An error caused the stream to end.
    Error,
}

/// Trait for types that can emit stream events.
pub trait EventEmitter {
    /// Emit a stream event to all listeners.
    fn emit(&self, event: StreamEvent);

    /// Returns `true` if the stream has been cancelled.
    fn is_cancelled(&self) -> bool;
}

/// Buffer for accumulating partial content from streaming deltas.
#[derive(Debug, Clone, Default)]
pub struct StreamBuffer {
    /// Accumulated text content.
    text: String,
    /// Accumulated thinking content.
    thinking: String,
    /// Active tool use inputs being accumulated.
    tool_inputs: Vec<ToolInputBuffer>,
    /// Total text deltas received.
    text_delta_count: usize,
    /// Total thinking deltas received.
    thinking_delta_count: usize,
}

/// Buffer for accumulating tool input JSON fragments.
#[derive(Debug, Clone)]
struct ToolInputBuffer {
    /// The tool use ID.
    tool_use_id: String,
    /// The tool name.
    #[allow(dead_code)]
    tool_name: String,
    /// Accumulated JSON fragments.
    json_fragments: String,
}

impl StreamBuffer {
    /// Create a new empty stream buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream event, accumulating content as appropriate.
    pub fn process(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::TextDelta { text } => {
                self.text.push_str(text);
                self.text_delta_count += 1;
            }
            StreamEvent::ThinkingDelta { text } => {
                self.thinking.push_str(text);
                self.thinking_delta_count += 1;
            }
            StreamEvent::ToolUseStart {
                tool_use_id,
                tool_name,
            } => {
                self.tool_inputs.push(ToolInputBuffer {
                    tool_use_id: tool_use_id.clone(),
                    tool_name: tool_name.clone(),
                    json_fragments: String::new(),
                });
            }
            StreamEvent::ToolInputDelta { tool_use_id, json } => {
                if let Some(buf) = self
                    .tool_inputs
                    .iter_mut()
                    .find(|b| b.tool_use_id == *tool_use_id)
                {
                    buf.json_fragments.push_str(json);
                }
            }
            _ => {}
        }
    }

    /// Returns the accumulated text content.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the accumulated thinking content.
    pub fn thinking(&self) -> &str {
        &self.thinking
    }

    /// Returns the number of text deltas received.
    pub fn text_delta_count(&self) -> usize {
        self.text_delta_count
    }

    /// Returns the number of thinking deltas received.
    pub fn thinking_delta_count(&self) -> usize {
        self.thinking_delta_count
    }

    /// Clear all accumulated content.
    pub fn clear(&mut self) {
        self.text.clear();
        self.thinking.clear();
        self.tool_inputs.clear();
        self.text_delta_count = 0;
        self.thinking_delta_count = 0;
    }
}
