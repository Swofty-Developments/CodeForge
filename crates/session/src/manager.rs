use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::claude::ClaudeSession;
use crate::codex::CodexSession;
use crate::types::AgentEvent;
use crate::{Provider, SessionId};

/// Wrapper around provider-specific session handles.
pub enum ActiveSession {
    Claude(ClaudeSession),
    Codex(CodexSession),
}

/// Manages multiple concurrent agent sessions.
pub struct SessionManager {
    sessions: HashMap<SessionId, ActiveSession>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Create and start a new agent session.
    ///
    /// Returns the session ID and a receiver for agent events.
    pub async fn create_session(
        &mut self,
        provider: Provider,
        cwd: &Path,
        model: Option<&str>,
    ) -> Result<(SessionId, mpsc::UnboundedReceiver<AgentEvent>)> {
        let id = uuid::Uuid::new_v4();

        let (active, event_rx) = match provider {
            Provider::ClaudeCode => {
                let (session, rx) = ClaudeSession::start(cwd, model)
                    .await
                    .context("Failed to start Claude Code session")?;
                (ActiveSession::Claude(session), rx)
            }
            Provider::Codex => {
                let (session, rx) = CodexSession::start(cwd, "codex")
                    .await
                    .context("Failed to start Codex session")?;
                (ActiveSession::Codex(session), rx)
            }
        };

        self.sessions.insert(id, active);
        Ok((id, event_rx))
    }

    /// Send a message to an existing session.
    pub async fn send_message(&mut self, session_id: SessionId, text: &str) -> Result<()> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .context("Session not found")?;

        match session {
            ActiveSession::Claude(s) => s.send_message(text),
            ActiveSession::Codex(s) => s.send_turn(text).await,
        }
    }

    /// Interrupt the current operation in a session.
    pub fn interrupt_session(&self, session_id: SessionId) -> Result<()> {
        let session = self.sessions.get(&session_id).context("Session not found")?;

        match session {
            ActiveSession::Claude(s) => s.interrupt(),
            ActiveSession::Codex(s) => s.interrupt(),
        }
    }

    /// Stop and remove a session.
    pub async fn stop_session(&mut self, session_id: SessionId) -> Result<()> {
        let mut session = self
            .sessions
            .remove(&session_id)
            .context("Session not found")?;

        match &mut session {
            ActiveSession::Claude(s) => s.stop().await,
            ActiveSession::Codex(s) => s.stop().await,
        }
    }

    /// Respond to an approval request in a Codex session.
    pub fn respond_to_approval(
        &self,
        session_id: SessionId,
        request_id: &str,
        approve: bool,
    ) -> Result<()> {
        let session = self.sessions.get(&session_id).context("Session not found")?;

        match session {
            ActiveSession::Codex(s) => s.respond_to_approval(request_id, approve),
            ActiveSession::Claude(_) => {
                anyhow::bail!("Approval responses are not supported for Claude Code sessions")
            }
        }
    }
}
