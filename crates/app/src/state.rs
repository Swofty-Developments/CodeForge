use codeforge_session::Provider;
use uuid::Uuid;

pub type ThreadId = Uuid;
pub type ProjectId = Uuid;

#[derive(Debug, Clone)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub path: String,
    pub threads: Vec<Thread>,
}

#[derive(Debug, Clone)]
pub struct Thread {
    pub id: ThreadId,
    pub title: String,
    pub provider: Provider,
    pub messages: Vec<ChatMessage>,
    pub is_active: bool,
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

#[derive(Debug)]
pub struct AppState {
    pub projects: Vec<Project>,
    pub open_tabs: Vec<ThreadId>,
    pub active_tab: Option<ThreadId>,
    pub sidebar_visible: bool,
    pub settings_open: bool,
    pub composer_text: String,
    pub selected_provider: Provider,
    pub approval_mode: ApprovalMode,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            projects: Vec::new(),
            open_tabs: Vec::new(),
            active_tab: None,
            sidebar_visible: true,
            settings_open: false,
            composer_text: String::new(),
            selected_provider: Provider::ClaudeCode,
            approval_mode: ApprovalMode::Supervised,
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
}
