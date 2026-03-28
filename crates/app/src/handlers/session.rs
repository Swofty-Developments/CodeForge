use uuid::Uuid;

use codeforge_session::SessionId;

use crate::message::Message;
use crate::state::{self, MessageRole, SessionState};
use crate::App;

pub fn handle_created(
    app: &mut App,
    thread_id: Uuid,
    session_id: SessionId,
    result: Result<(), String>,
) -> iced::Task<Message> {
    let state = &mut app.state;

    match result {
        Ok(()) => {
            state.thread_sessions.insert(thread_id, session_id);
            state
                .session_states
                .insert(session_id, SessionState::Starting);
            tracing::info!("Session {session_id} created for thread {thread_id}");
        }
        Err(e) => {
            tracing::error!("Failed to create session for thread {thread_id}: {e}");
            if let Some(thread) = state.find_thread_mut(thread_id) {
                thread.messages.push(state::ChatMessage {
                    id: Uuid::new_v4(),
                    role: MessageRole::System,
                    content: format!("Failed to start agent session: {e}"),
                });
            }
        }
    }

    iced::Task::none()
}

pub fn handle_message_sent(
    app: &mut App,
    thread_id: Uuid,
    result: Result<(), String>,
) -> iced::Task<Message> {
    if let Err(e) = result {
        tracing::error!("Failed to send message to session: {e}");
        if let Some(thread) = app.state.find_thread_mut(thread_id) {
            thread.messages.push(state::ChatMessage {
                id: Uuid::new_v4(),
                role: MessageRole::System,
                content: format!("Failed to send message: {e}"),
            });
        }
    }
    iced::Task::none()
}
