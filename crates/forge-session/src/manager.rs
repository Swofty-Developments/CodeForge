use std::collections::HashMap;

use forge_core::{SessionInfo, SessionStatus, StartSessionOpts};
use tokio::sync::mpsc;
use tracing::warn;
use uuid::Uuid;

use crate::{AgentEvent, ClaudeSession, Result};

/// Registry entry: the live session plus display metadata for `list()`.
struct SessionEntry {
    session: ClaudeSession,
    title: String,
    model: Option<String>,
    /// Resume id requested at start; superseded by the sidecar-confirmed id.
    thread_hint: Option<String>,
}

/// Multi-session registry. Owned by the Tauri app behind a `tokio::sync::Mutex`.
///
/// Session ids cross IPC as strings (uuid v4, parsed here).
#[derive(Default)]
pub struct SessionManager {
    sessions: HashMap<Uuid, SessionEntry>,
    next_seq: u64,
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
        let id = Uuid::new_v4();
        let mut thread_hint = opts.resume_session_id.clone();

        let session = match &opts.resume_session_id {
            Some(resume_id) => {
                match ClaudeSession::resume(&opts.repo_path, resume_id, opts.model.as_deref(), event_tx.clone())
                    .await
                {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("failed to resume claude session {resume_id}, starting fresh: {e}");
                        thread_hint = None;
                        Self::start_fresh(&opts, event_tx).await?
                    }
                }
            }
            None => Self::start_fresh(&opts, event_tx).await?,
        };

        self.next_seq += 1;
        let title = format!("Session {}", self.next_seq);
        let info = SessionInfo {
            id: id.to_string(),
            thread_id: thread_hint.clone(),
            title: title.clone(),
            model: opts.model.clone(),
            status: SessionStatus::Starting,
        };
        self.sessions
            .insert(id, SessionEntry { session, title, model: opts.model, thread_hint });
        Ok(info)
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
        let mut entry = self
            .sessions
            .remove(&uuid)
            .ok_or_else(|| crate::Error::NotFound(id.to_string()))?;
        entry.session.stop().await
    }

    /// Snapshot of all live sessions (for the `list_sessions` command).
    /// Live turn state (generating/error) is frontend-tracked via events; this
    /// snapshot only distinguishes pre-/post-handshake.
    pub fn list(&self) -> Vec<SessionInfo> {
        self.sessions
            .iter()
            .map(|(id, entry)| {
                let confirmed = entry.session.claude_session_id();
                let status = if confirmed.is_some() { SessionStatus::Ready } else { SessionStatus::Starting };
                SessionInfo {
                    id: id.to_string(),
                    thread_id: confirmed.or_else(|| entry.thread_hint.clone()),
                    title: entry.title.clone(),
                    model: entry.model.clone(),
                    status,
                }
            })
            .collect()
    }

    async fn start_fresh(opts: &StartSessionOpts, event_tx: mpsc::Sender<AgentEvent>) -> Result<ClaudeSession> {
        ClaudeSession::start(
            &opts.repo_path,
            opts.model.as_deref(),
            opts.permission_mode.as_deref(),
            event_tx,
        )
        .await
    }

    fn get(&self, id: &str) -> Result<&ClaudeSession> {
        let uuid = parse_id(id)?;
        self.sessions
            .get(&uuid)
            .map(|entry| &entry.session)
            .ok_or_else(|| crate::Error::NotFound(id.to_string()))
    }
}

fn parse_id(id: &str) -> Result<Uuid> {
    id.parse::<Uuid>()
        .map_err(|_| crate::Error::NotFound(format!("invalid session id: {id}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_and_invalid_ids_are_not_found() {
        let mut mgr = SessionManager::new();
        assert!(mgr.list().is_empty());
        assert!(matches!(mgr.send("not-a-uuid", "hi"), Err(crate::Error::NotFound(_))));
        let ghost = Uuid::new_v4().to_string();
        assert!(matches!(mgr.abort(&ghost), Err(crate::Error::NotFound(_))));
        assert!(matches!(mgr.approve(&ghost, "1", true), Err(crate::Error::NotFound(_))));
        assert!(matches!(mgr.stop(&ghost).await, Err(crate::Error::NotFound(_))));
    }
}
