//! Tauri event channel names (frozen contract — mirrored in frontend/src/ipc.ts).

#![allow(dead_code)] // scaffold: consumed once command bodies are implemented

/// All session streaming: payload `forge_session::AgentEventPayload`, demuxed
/// frontend-side by `sessionId`.
pub const AGENT_EVENT: &str = "agent-event";

/// Live timeline appends: payload `forge_core::TimelineEvent`.
pub const TIMELINE_EVENT: &str = "timeline:event";

/// Indexing progress: payload `forge_core::IndexProgress`.
pub const INDEX_PROGRESS: &str = "index:progress";

/// Repo opened/closed/re-indexed: payload `forge_core::RepoState`.
pub const REPO_CHANGED: &str = "repo:changed";

/// Per-session status changes: payload `forge_core::SessionInfo`.
pub fn session_status_event(session_id: &str) -> String {
    format!("session:{session_id}:status")
}
