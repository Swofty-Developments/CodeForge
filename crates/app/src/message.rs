use crate::state::{ApprovalMode, ProjectId, ThreadId};
use codeforge_session::{AgentEvent, Provider, SessionId};

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(SidebarMessage),
    Chat(ChatMessage),
    Composer(ComposerMessage),
    Tab(TabMessage),
    Settings(SettingsMessage),
    Agent(AgentMessage),
    Popup(PopupMessage),
    DbLoaded(Result<DbPayload, String>),
    SessionCreated {
        thread_id: ThreadId,
        session_id: SessionId,
        result: Result<(), String>,
    },
    MessageSent {
        thread_id: ThreadId,
        result: Result<(), String>,
    },
}

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
    NewThreadInProject(ProjectId),
    DeleteThread(ThreadId),
    ToggleSidebar,
    // Thread rename
    StartRename(ThreadId),
    RenameTextChanged(String),
    ConfirmRename,
    CancelRename,
    // Thread context menu
    ShowThreadContextMenu(ThreadId),
    ShowProjectContextMenu(ProjectId),
    CloseContextMenu,
    // Thread color
    SetThreadColor(ThreadId, Option<String>),
    // Project/group management
    RenameProject(ProjectId),
    ProjectRenameTextChanged(String),
    ConfirmProjectRename,
    CancelProjectRename,
    SetProjectColor(ProjectId, Option<String>),
    DeleteProject(ProjectId),
    ToggleProjectCollapse(ProjectId),
    // Drag thread
    StartDragThread(ThreadId),
    DropOnProject(ProjectId),
    CancelDrag,
    // Sidebar resize
    StartResize,
    Resize(f32),
    EndResize,
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
    ToggleProviderPicker,
    PickFolder,
    FolderPicked(Option<String>),
    ConfirmNewGroup(String),
}

#[derive(Debug, Clone)]
pub enum TabMessage {
    Select(ThreadId),
    Close(ThreadId),
    StartDrag(ThreadId),
    DragOver(usize),
    EndDrag,
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    Open,
    Close,
    ApprovalModeChanged(ApprovalMode),
    ClaudePathChanged(String),
    CodexPathChanged(String),
}

#[derive(Debug, Clone)]
pub enum PopupMessage {
    ConfirmDeleteGroup { delete_threads: bool },
    CancelPopup,
}
