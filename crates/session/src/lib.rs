use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
