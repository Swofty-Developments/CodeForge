use crate::message::{Message, SettingsMessage};
use crate::state;
use crate::App;

pub fn handle(app: &mut App, msg: SettingsMessage) -> iced::Task<Message> {
    let state = &mut app.state;

    match msg {
        SettingsMessage::Open => {
            state.settings_open = true;
            state.context_menu = None;
        }
        SettingsMessage::Close => state.settings_open = false,
        SettingsMessage::ApprovalModeChanged(mode) => {
            state.approval_mode = mode;
            if let Some(ref db) = app.db {
                if let Ok(db) = db.lock() {
                    let val = match mode {
                        state::ApprovalMode::AutoApprove => "auto_approve",
                        state::ApprovalMode::Supervised => "supervised",
                    };
                    let _ = codeforge_persistence::queries::set_setting(
                        db.conn(),
                        "approval_mode",
                        val,
                    );
                }
            }
        }
        SettingsMessage::ClaudePathChanged(path) => {
            state.claude_path = path.clone();
            if let Some(ref db) = app.db {
                if let Ok(db) = db.lock() {
                    let _ = codeforge_persistence::queries::set_setting(
                        db.conn(),
                        "claude_path",
                        &path,
                    );
                }
            }
        }
        SettingsMessage::CodexPathChanged(path) => {
            state.codex_path = path.clone();
            if let Some(ref db) = app.db {
                if let Ok(db) = db.lock() {
                    let _ = codeforge_persistence::queries::set_setting(
                        db.conn(),
                        "codex_path",
                        &path,
                    );
                }
            }
        }
    }

    iced::Task::none()
}
