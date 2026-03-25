use crate::state::{ApprovalMode, ThreadId};
use codeforge_session::{AgentEvent, Provider, SessionId};

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(SidebarMessage),
    Chat(ChatMessage),
    Composer(ComposerMessage),
    Tab(TabMessage),
    Settings(SettingsMessage),
    Agent(AgentMessage),
    /// Async initialization result
    DbLoaded(Result<DbPayload, String>),
    /// A session was created asynchronously
    SessionCreated {
        thread_id: ThreadId,
        session_id: SessionId,
        result: Result<(), String>,
    },
    /// A message was sent to the session asynchronously
    MessageSent {
        thread_id: ThreadId,
        result: Result<(), String>,
    },
}

/// Payload from DB load at startup
#[derive(Debug, Clone)]
pub struct DbPayload {
    pub projects: Vec<codeforge_persistence::Project>,
    pub threads_by_project: Vec<(uuid::Uuid, Vec<codeforge_persistence::Thread>)>,
    pub messages_by_thread: Vec<(uuid::Uuid, Vec<codeforge_persistence::Message>)>,
}

#[derive(Debug, Clone)]
pub enum AgentMessage {
    Event {
        session_id: SessionId,
        event: AgentEvent,
    },
    StartSession {
        thread_id: ThreadId,
    },
    StopSession {
        thread_id: ThreadId,
    },
    ApprovalResponse {
        session_id: SessionId,
        request_id: String,
        approve: bool,
    },
}

#[derive(Debug, Clone)]
pub enum SidebarMessage {
    SelectThread(ThreadId),
    NewThread,
    DeleteThread(ThreadId),
    ToggleSidebar,
}

#[derive(Debug, Clone)]
pub enum ChatMessage {
    ApproveRequest { request_id: String },
    DenyRequest { request_id: String },
}

#[derive(Debug, Clone)]
pub enum ComposerMessage {
    TextChanged(String),
    Send,
    ProviderChanged(Provider),
}

#[derive(Debug, Clone)]
pub enum TabMessage {
    Select(ThreadId),
    Close(ThreadId),
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    Open,
    Close,
    ApprovalModeChanged(ApprovalMode),
    ClaudePathChanged(String),
    CodexPathChanged(String),
}
