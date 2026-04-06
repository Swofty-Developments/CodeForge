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
        permission_mode: Option<&str>,
    ) -> Result<(SessionId, mpsc::Receiver<AgentEvent>)> {
        let id = uuid::Uuid::new_v4();

        let (active, event_rx) = match provider {
            Provider::ClaudeCode => {
                let (session, rx) = ClaudeSession::start(cwd, model, permission_mode)
                    .await
                    .context("Failed to start Claude Code session")?;
                (ActiveSession::Claude(session), rx)
            }
            Provider::Codex => {
                let (session, mut unbounded_rx) = CodexSession::start(cwd, "codex")
                    .await
                    .context("Failed to start Codex session")?;
                // Bridge unbounded → bounded channel for uniform return type.
                let (tx, rx) = mpsc::channel(1024);
                tokio::spawn(async move {
                    while let Some(event) = unbounded_rx.recv().await {
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                });
                (ActiveSession::Codex(session), rx)
            }
        };

        self.sessions.insert(id, active);
        Ok((id, event_rx))
    }

    /// Resume a previous Claude Code session using `--resume`.
    ///
    /// Returns the new internal session ID and event receiver.
    pub async fn resume_session(
        &mut self,
        claude_session_id: &str,
        cwd: &Path,
        model: Option<&str>,
    ) -> Result<(SessionId, mpsc::Receiver<AgentEvent>)> {
        let id = uuid::Uuid::new_v4();

        let (session, rx) = ClaudeSession::resume(cwd, claude_session_id, model)
            .await
            .context("Failed to resume Claude Code session")?;

        self.sessions.insert(id, ActiveSession::Claude(session));
        Ok((id, rx))
    }

    /// Get the Claude CLI session ID for an active session, if available.
    pub fn claude_session_id(&self, session_id: SessionId) -> Option<String> {
        match self.sessions.get(&session_id) {
            Some(ActiveSession::Claude(s)) => s.claude_session_id(),
            _ => None,
        }
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

    /// Respond to an approval request in an agent session.
    pub fn respond_to_approval(
        &self,
        session_id: SessionId,
        request_id: &str,
        approve: bool,
    ) -> Result<()> {
        let session = self.sessions.get(&session_id).context("Session not found")?;

        match session {
            ActiveSession::Codex(s) => s.respond_to_approval(request_id, approve),
            ActiveSession::Claude(s) => s.respond_to_approval(request_id, approve, None),
        }
    }
}
