//! forge-session — Claude Code session lifecycle via the Node agent-sidecar.
//!
//! One sidecar process (`crates/tauri-app/agent-sidecar/index.mjs`, wrapping
//! `@anthropic-ai/claude-agent-sdk`) per session, speaking NDJSON over
//! stdin/stdout. See docs/ARCHITECTURE.md §Sessions and the sidecar's protocol
//! doc-comment; [`AgentEvent`] mirrors the sidecar out-events 1:1.

mod claude;
mod locate;
mod manager;
mod payload;
mod protocol;
pub mod shell_env;
mod types;

pub use claude::ClaudeSession;
pub use manager::SessionManager;
pub use payload::AgentEventPayload;
pub use types::AgentEvent;

/// Session errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("sidecar error: {0}")]
    Sidecar(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
