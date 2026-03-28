use crate::message::{ChatMessage, Message};
use crate::App;

pub fn handle(app: &mut App, msg: ChatMessage) -> iced::Task<Message> {
    let state = &mut app.state;

    match msg {
        ChatMessage::ApproveRequest { request_id } => {
            if let Some(active_tab) = state.active_tab {
                if let Some(&session_id) = state.thread_sessions.get(&active_tab) {
                    let mgr = app.session_manager.clone();
                    let rid = request_id.clone();
                    tokio::spawn(async move {
                        let mgr = mgr.lock().await;
                        if let Err(e) = mgr.respond_to_approval(session_id, &rid, true) {
                            tracing::error!("Failed to approve: {e:#}");
                        }
                    });
                    state
                        .pending_approvals
                        .retain(|a| a.request_id != request_id);
                }
            }
        }
        ChatMessage::DenyRequest { request_id } => {
            if let Some(active_tab) = state.active_tab {
                if let Some(&session_id) = state.thread_sessions.get(&active_tab) {
                    let mgr = app.session_manager.clone();
                    let rid = request_id.clone();
                    tokio::spawn(async move {
                        let mgr = mgr.lock().await;
                        if let Err(e) = mgr.respond_to_approval(session_id, &rid, false) {
                            tracing::error!("Failed to deny: {e:#}");
                        }
                    });
                    state
                        .pending_approvals
                        .retain(|a| a.request_id != request_id);
                }
            }
        }
    }

    iced::Task::none()
}
