use crate::state::{ApprovalMode, ThreadId};
use codeforge_session::Provider;

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(SidebarMessage),
    Chat(ChatMessage),
    Composer(ComposerMessage),
    Tab(TabMessage),
    Settings(SettingsMessage),
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
    // Future: scroll, copy, etc.
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
}
