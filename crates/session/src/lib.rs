use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod claude;
pub mod codex;
pub mod manager;
pub mod protocol;
pub mod types;

/// Unique identifier for a session.
pub type SessionId = Uuid;

/// The AI agent provider for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    ClaudeCode,
    Codex,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::ClaudeCode => write!(f, "Claude Code"),
            Provider::Codex => write!(f, "Codex"),
        }
    }
}

// Re-export key types at the crate root for convenience.
pub use claude::ClaudeSession;
pub use codex::CodexSession;
pub use manager::{ActiveSession, SessionManager};
pub use protocol::{
    parse_jsonrpc_line, JsonRpcError, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse,
};
pub use types::{AgentEvent, ApprovalMode, Session, SessionStatus};
