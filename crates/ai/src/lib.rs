//! AI provider abstraction and message types for CodeForge.
//!
//! Defines the traits and data structures for interacting with AI model
//! backends, managing conversation messages, tool definitions, context
//! windows, cost tracking, and streaming responses.

pub mod agent;
pub mod cache;
pub mod context;
pub mod cost;
pub mod eval;
pub mod message;
pub mod prompt;
pub mod retry;
pub mod streaming;
pub mod token;
pub mod tool;

// The provider module uses trait_variant which requires the crate.
// Since trait_variant may not be available, we define the provider
// types without the async trait macro.
pub mod provider;

pub use context::{CompactionStrategy, ContextManager, ContextWindow};
pub use cost::{CostTracker, UsageReport};
pub use message::{ContentBlock, Message, StoredMessage};
pub use provider::{SessionConfig, SessionHandle};
pub use streaming::{EventEmitter, StopReason, StreamBuffer, StreamEvent};
pub use tool::{ToolApproval, ToolCategory, ToolDefinition, ToolResult};
