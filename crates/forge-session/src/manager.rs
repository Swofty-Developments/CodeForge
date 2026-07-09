use std::collections::HashMap;

use forge_core::{SessionInfo, StartSessionOpts};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{AgentEvent, ClaudeSession, Result};

/// Multi-session registry. Owned by the Tauri app behind a `tokio::sync::Mutex`.
///
/// Session ids cross IPC as strings (uuid v4, parsed here).
#[derive(Default)]
pub struct SessionManager {
    sessions: HashMap<Uuid, ClaudeSession>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a session per `opts` (resume when `opts.resume_session_id` is set,
    /// with fresh-session fallback). Events flow to `event_tx`; the caller
    /// forwards them to the frontend as `agent-event` payloads.
    pub async fn start_session(
        &mut self,
        opts: StartSessionOpts,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<SessionInfo> {
        let _ = (opts, event_tx);
        // IMPLEMENT(agent): Uuid::new_v4 id; ClaudeSession::resume(...) fallback
        // to ClaudeSession::start(...); insert; return SessionInfo{status: Starting}.
        todo!("SessionManager::start_session")
    }

    /// Queue a user prompt on an existing session.
    pub fn send(&self, id: &str, text: &str) -> Result<()> {
        self.get(id)?.send_message(text)
    }

    /// Answer a pending approval request on a session.
    pub fn approve(&self, id: &str, request_id: &str, approve: bool) -> Result<()> {
        self.get(id)?.respond_to_approval(request_id, approve, None)
    }

    /// Abort the in-flight turn (session stays alive).
    pub fn abort(&self, id: &str) -> Result<()> {
        self.get(id)?.interrupt()
    }

    /// Stop and remove a session (kills the sidecar).
    pub async fn stop(&mut self, id: &str) -> Result<()> {
        let uuid = parse_id(id)?;
        let mut session = self
            .sessions
            .remove(&uuid)
            .ok_or_else(|| crate::Error::NotFound(id.to_string()))?;
        session.stop().await
    }

    /// Snapshot of all live sessions (for the `list_sessions` command).
    pub fn list(&self) -> Vec<SessionInfo> {
        // IMPLEMENT(agent): map sessions to SessionInfo (title/model/status
        // tracked alongside ClaudeSession in the registry).
        todo!("SessionManager::list")
    }

    fn get(&self, id: &str) -> Result<&ClaudeSession> {
        let uuid = parse_id(id)?;
        self.sessions
            .get(&uuid)
            .ok_or_else(|| crate::Error::NotFound(id.to_string()))
    }
}

fn parse_id(id: &str) -> Result<Uuid> {
    id.parse::<Uuid>()
        .map_err(|_| crate::Error::NotFound(format!("invalid session id: {id}")))
}
