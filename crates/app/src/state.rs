use std::collections::HashMap;

use codeforge_session::{Provider, SessionId};
use uuid::Uuid;

pub type ThreadId = Uuid;
pub type ProjectId = Uuid;

#[derive(Debug, Clone)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub path: String,
    pub color: Option<String>,
    pub collapsed: bool,
    pub threads: Vec<Thread>,
}

#[derive(Debug, Clone)]
pub struct Thread {
    pub id: ThreadId,
    pub title: String,
    pub color: Option<String>,
    pub provider: Provider,
    pub messages: Vec<ChatMessage>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub enum ContextMenu {
    Thread(ThreadId),
    Project(ProjectId),
}

#[derive(Debug, Clone)]
pub enum PendingPopup {
    ConfirmNewGroup { directory: String },
    ConfirmDeleteGroup { project_id: ProjectId },
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalMode {
    Supervised,
    AutoApprove,
}

impl std::fmt::Display for ApprovalMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalMode::Supervised => write!(f, "Supervised"),
            ApprovalMode::AutoApprove => write!(f, "Auto-approve"),
        }
    }
}

/// A pending approval request from an agent
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub session_id: SessionId,
    pub request_id: String,
    pub description: String,
}

/// Status of a session associated with a thread
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Starting,
    Ready,
    Generating,
    Error,
}

pub const SIDEBAR_MIN_WIDTH: f32 = 160.0;
pub const SIDEBAR_MAX_WIDTH: f32 = 500.0;
pub const SIDEBAR_DEFAULT_WIDTH: f32 = 240.0;

#[derive(Debug)]
pub struct AppState {
    pub projects: Vec<Project>,
    pub open_tabs: Vec<ThreadId>,
    pub active_tab: Option<ThreadId>,
    pub sidebar_visible: bool,
    pub sidebar_width: f32,
    pub sidebar_dragging: bool,
    pub settings_open: bool,
    pub composer_text: String,
    pub selected_provider: Provider,
    pub approval_mode: ApprovalMode,
    // Session tracking
    pub thread_sessions: HashMap<ThreadId, SessionId>,
    pub session_states: HashMap<SessionId, SessionState>,
    pub pending_approvals: Vec<PendingApproval>,
    /// Tracks whether we have a streaming assistant message being built
    pub streaming_threads: HashMap<ThreadId, Uuid>,
    // Settings: binary paths
    pub claude_path: String,
    pub codex_path: String,
    // DB loaded flag
    pub db_loaded: bool,
    pub renaming_thread: Option<(ThreadId, String)>,
    pub renaming_project: Option<(ProjectId, String)>,
    pub context_menu: Option<ContextMenu>,
    pub pending_popup: Option<PendingPopup>,
    pub provider_picker_open: bool,
    pub dragging_thread: Option<ThreadId>,
    pub dragging_tab: Option<ThreadId>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            projects: Vec::new(),
            open_tabs: Vec::new(),
            active_tab: None,
            sidebar_visible: true,
            sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            sidebar_dragging: false,
            settings_open: false,
            composer_text: String::new(),
            selected_provider: Provider::ClaudeCode,
            approval_mode: ApprovalMode::Supervised,
            thread_sessions: HashMap::new(),
            session_states: HashMap::new(),
            pending_approvals: Vec::new(),
            streaming_threads: HashMap::new(),
            claude_path: "claude".to_string(),
            codex_path: "codex".to_string(),
            db_loaded: false,
            renaming_thread: None,
            renaming_project: None,
            context_menu: None,
            pending_popup: None,
            provider_picker_open: false,
            dragging_thread: None,
            dragging_tab: None,
        }
    }
}

impl AppState {
    pub fn active_thread(&self) -> Option<&Thread> {
        let tab_id = self.active_tab?;
        self.projects
            .iter()
            .flat_map(|p| &p.threads)
            .find(|t| t.id == tab_id)
    }

    pub fn active_thread_mut(&mut self) -> Option<&mut Thread> {
        let tab_id = self.active_tab?;
        self.projects
            .iter_mut()
            .flat_map(|p| &mut p.threads)
            .find(|t| t.id == tab_id)
    }

    pub fn find_thread(&self, id: ThreadId) -> Option<&Thread> {
        self.projects
            .iter()
            .flat_map(|p| &p.threads)
            .find(|t| t.id == id)
    }

    pub fn find_thread_mut(&mut self, id: ThreadId) -> Option<&mut Thread> {
        self.projects
            .iter_mut()
            .flat_map(|p| &mut p.threads)
            .find(|t| t.id == id)
    }

    /// Check if a thread currently has a generating (streaming) session
    pub fn is_thread_generating(&self, thread_id: ThreadId) -> bool {
        if let Some(session_id) = self.thread_sessions.get(&thread_id) {
            self.session_states.get(session_id) == Some(&SessionState::Generating)
        } else {
            false
        }
    }

    /// Check if a thread has an active session (any state)
    pub fn has_active_session(&self, thread_id: ThreadId) -> bool {
        self.thread_sessions.contains_key(&thread_id)
    }

    /// Get session state for a thread
    pub fn thread_session_state(&self, thread_id: ThreadId) -> Option<SessionState> {
        let session_id = self.thread_sessions.get(&thread_id)?;
        self.session_states.get(session_id).copied()
    }

    /// Get pending approvals for the active thread
    pub fn is_uncategorized(&self, thread_id: ThreadId) -> bool {
        self.projects
            .iter()
            .find(|p| p.threads.iter().any(|t| t.id == thread_id))
            .map(|p| p.path == ".")
            .unwrap_or(true)
    }

    pub fn uncategorized_project_id(&self) -> Option<ProjectId> {
        self.projects.iter().find(|p| p.path == ".").map(|p| p.id)
    }

    pub fn active_thread_approvals(&self) -> Vec<&PendingApproval> {
        let Some(thread_id) = self.active_tab else {
            return Vec::new();
        };
        let Some(session_id) = self.thread_sessions.get(&thread_id) else {
            return Vec::new();
        };
        self.pending_approvals
            .iter()
            .filter(|a| &a.session_id == session_id)
            .collect()
    }
}
