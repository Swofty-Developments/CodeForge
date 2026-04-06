use chrono::{DateTime, Utc};
use codeforge_core::id::{MessageId, ProjectId, SessionId, ThreadId, WorktreeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub path: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: ThreadId,
    pub project_id: ProjectId,
    pub title: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            other => Err(format!("invalid message role: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub thread_id: ThreadId,
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Codex,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            other => Err(format!("invalid provider: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub thread_id: ThreadId,
    pub provider: Provider,
    pub status: String,
    pub approval_mode: Option<String>,
    pub pid: Option<i64>,
    pub claude_session_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorktreeStatus {
    /// Worktree exists on disk and is available for work.
    Active,
    /// PR was merged on GitHub and the change is reachable from the base branch.
    /// Thread is read-only.
    Merged,
    /// PR was closed on GitHub without being merged. Thread is read-only.
    Closed,
    /// User explicitly detached this worktree — thread is free to create a new one.
    Deleted,
    /// Worktree directory/record is missing from git but the DB row survives.
    Orphaned,
}

impl WorktreeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Merged => "merged",
            Self::Closed => "closed",
            Self::Deleted => "deleted",
            Self::Orphaned => "orphaned",
        }
    }

    /// True if the worktree still accepts work (composer enabled, button active).
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Active)
    }
}

impl std::str::FromStr for WorktreeStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "merged" => Ok(Self::Merged),
            "closed" => Ok(Self::Closed),
            "deleted" => Ok(Self::Deleted),
            "orphaned" => Ok(Self::Orphaned),
            other => Err(format!("invalid worktree status: {other}")),
        }
    }
}

/// Last-observed GitHub PR state — mirrors `gh pr view --json state`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrGhState {
    Open,
    Closed,
    Merged,
    Unknown,
}

impl PrGhState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Merged => "merged",
            Self::Unknown => "unknown",
        }
    }
}

impl std::str::FromStr for PrGhState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            "merged" => Ok(Self::Merged),
            _ => Ok(Self::Unknown),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    pub id: WorktreeId,
    pub thread_id: ThreadId,
    pub project_id: ProjectId,
    pub branch: String,
    pub path: String,
    pub pr_number: Option<u32>,
    pub status: WorktreeStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Last-observed GitHub PR state (open/closed/merged/unknown).
    pub pr_state: Option<PrGhState>,
    /// Merge commit SHA when the PR has been merged — used for revert detection.
    pub pr_merge_commit: Option<String>,
    /// Number of PR review comments seen by the poller, persisted across restarts
    /// so we can compute a true delta and not replay history or lose comments.
    pub last_seen_comment_count: u32,
    /// Cached PR URL for quick access (banner, clickable links).
    pub pr_url: Option<String>,
}
