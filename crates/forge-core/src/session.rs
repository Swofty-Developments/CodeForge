use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Returned by `open_repo`; describes an opened repository.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoState {
    pub path: PathBuf,
    pub name: String,
    pub features_count: u32,
    /// `None` until the first cold-start index completes.
    pub indexed_at: Option<DateTime<Utc>>,
    /// `None` if the per-repo daemon failed to start.
    pub daemon_port: Option<u16>,
    /// Current git branch (title-bar pill). `None` on a detached HEAD.
    #[serde(default)]
    pub branch: Option<String>,
}

/// Streamed to the frontend via the `index:progress` event during indexing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    /// e.g. "decompose" | "docs" | "classify".
    pub stage: String,
    /// Human-readable detail line (current feature/file).
    pub detail: String,
    pub done: u32,
    pub total: u32,
}

/// Returned by `start_session` / `list_sessions`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    /// Session-manager id (uuid, stringly-typed across IPC).
    pub id: String,
    /// Claude Agent SDK session id (for `--resume`), once known.
    pub thread_id: Option<String>,
    pub title: String,
    pub model: Option<String>,
    pub status: SessionStatus,
}

/// Lifecycle status of an embedded Claude Code session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Starting,
    Ready,
    Generating,
    Error,
    Stopped,
}

/// Options for `start_session`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionOpts {
    pub repo_path: PathBuf,
    /// Model alias or full name; `None` = CLI default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// SDK permissionMode: "default" | "acceptEdits" | "plan" | "bypassPermissions".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Claude session id to resume, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_session_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_state_roundtrip() {
        let r = RepoState {
            path: PathBuf::from("/tmp/repo"),
            name: "repo".into(),
            features_count: 7,
            indexed_at: Some(Utc::now()),
            daemon_port: Some(49213),
            branch: Some("main".into()),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"featuresCount\""));
        assert!(json.contains("\"daemonPort\""));
        let back: RepoState = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn session_info_roundtrip() {
        let s = SessionInfo {
            id: "6f9619ff-8b86-4d01-b42d-00cf4fc964ff".into(),
            thread_id: None,
            title: "Session 1".into(),
            model: Some("sonnet".into()),
            status: SessionStatus::Generating,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"status\":\"generating\""));
        assert!(json.contains("\"threadId\":null"));
        let back: SessionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn start_session_opts_minimal() {
        let o: StartSessionOpts = serde_json::from_str(r#"{"repoPath":"/tmp/repo"}"#).unwrap();
        assert_eq!(o.repo_path, PathBuf::from("/tmp/repo"));
        assert!(o.model.is_none() && o.permission_mode.is_none() && o.resume_session_id.is_none());
        let json = serde_json::to_string(&o).unwrap();
        assert_eq!(json, r#"{"repoPath":"/tmp/repo"}"#);
    }

    #[test]
    fn index_progress_roundtrip() {
        let p = IndexProgress { stage: "docs".into(), detail: "auth-flow".into(), done: 2, total: 9 };
        let back: IndexProgress = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(p, back);
    }
}
