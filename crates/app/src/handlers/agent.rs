use uuid::Uuid;

use codeforge_session::{AgentEvent, SessionId};

use crate::message::{AgentMessage, Message};
use crate::state::{self, MessageRole, PendingApproval, SessionState};
use crate::App;

pub fn handle(app: &mut App, msg: AgentMessage) -> iced::Task<Message> {
    match msg {
        AgentMessage::Event { session_id, event } => handle_event(app, session_id, event),
        AgentMessage::StartSession { thread_id } => handle_start(app, thread_id),
        AgentMessage::StopSession { thread_id } => {
            handle_stop(app, thread_id);
            iced::Task::none()
        }
        AgentMessage::ApprovalResponse {
            session_id,
            request_id,
            approve,
        } => {
            handle_approval(app, session_id, &request_id, approve);
            iced::Task::none()
        }
    }
}

fn handle_event(
    app: &mut App,
    session_id: SessionId,
    event: AgentEvent,
) -> iced::Task<Message> {
    let state = &mut app.state;

    let thread_id = state
        .thread_sessions
        .iter()
        .find(|(_, &sid)| sid == session_id)
        .map(|(&tid, _)| tid);

    let Some(thread_id) = thread_id else {
        tracing::warn!("Received agent event for unknown session {session_id}");
        return iced::Task::none();
    };

    match event {
        AgentEvent::ContentDelta { text } => {
            if let Some(&msg_id) = state.streaming_threads.get(&thread_id) {
                if let Some(thread) = state.find_thread_mut(thread_id) {
                    if let Some(msg) = thread.messages.iter_mut().find(|m| m.id == msg_id) {
                        msg.content.push_str(&text);
                    }
                }
            } else {
                let msg_id = Uuid::new_v4();
                state.streaming_threads.insert(thread_id, msg_id);
                if let Some(thread) = state.find_thread_mut(thread_id) {
                    thread.messages.push(state::ChatMessage {
                        id: msg_id,
                        role: MessageRole::Assistant,
                        content: text,
                    });
                }
            }
        }
        AgentEvent::TurnStarted { .. } => {
            state
                .session_states
                .insert(session_id, SessionState::Generating);
        }
        AgentEvent::TurnCompleted { .. } => {
            state
                .session_states
                .insert(session_id, SessionState::Ready);

            if let Some(msg_id) = state.streaming_threads.remove(&thread_id) {
                if let Some(thread) = state.find_thread(thread_id) {
                    if let Some(msg) = thread.messages.iter().find(|m| m.id == msg_id) {
                        if let Some(ref db) = app.db {
                            if let Ok(db) = db.lock() {
                                let db_msg = codeforge_persistence::Message {
                                    id: msg_id,
                                    thread_id,
                                    role: codeforge_persistence::MessageRole::Assistant,
                                    content: msg.content.clone(),
                                    created_at: chrono::Utc::now(),
                                };
                                if let Err(e) =
                                    codeforge_persistence::queries::insert_message(
                                        db.conn(),
                                        &db_msg,
                                    )
                                {
                                    tracing::error!(
                                        "Failed to persist assistant message: {e:#}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        AgentEvent::TurnAborted { reason } => {
            state
                .session_states
                .insert(session_id, SessionState::Ready);
            state.streaming_threads.remove(&thread_id);

            if let Some(thread) = state.find_thread_mut(thread_id) {
                thread.messages.push(state::ChatMessage {
                    id: Uuid::new_v4(),
                    role: MessageRole::System,
                    content: format!("Turn aborted: {reason}"),
                });
            }
        }
        AgentEvent::ApprovalRequired {
            request_id,
            description,
        } => {
            state.pending_approvals.push(PendingApproval {
                session_id,
                request_id,
                description,
            });
        }
        AgentEvent::SessionReady => {
            state
                .session_states
                .insert(session_id, SessionState::Ready);
        }
        AgentEvent::SessionError { message } => {
            state
                .session_states
                .insert(session_id, SessionState::Error);

            if let Some(thread) = state.find_thread_mut(thread_id) {
                thread.messages.push(state::ChatMessage {
                    id: Uuid::new_v4(),
                    role: MessageRole::System,
                    content: format!("Session error: {message}"),
                });
            }
        }
    }

    iced::Task::none()
}

fn handle_start(app: &mut App, thread_id: uuid::Uuid) -> iced::Task<Message> {
    let state = &mut app.state;

    if state.thread_sessions.contains_key(&thread_id) {
        return iced::Task::none();
    }

    let provider = state
        .find_thread(thread_id)
        .map(|t| t.provider)
        .unwrap_or(state.selected_provider);
    let mgr = app.session_manager.clone();
    let receivers = app.event_receivers.clone();
    let cwd = state
        .projects
        .first()
        .map(|p| p.path.clone())
        .unwrap_or_else(|| ".".to_string());

    iced::Task::perform(
        async move {
            let cwd_path = std::path::PathBuf::from(&cwd);
            let mut mgr = mgr.lock().await;
            match mgr.create_session(provider, &cwd_path).await {
                Ok((session_id, event_rx)) => {
                    receivers.insert(session_id, event_rx).await;
                    Ok(session_id)
                }
                Err(e) => Err(format!("{e:#}")),
            }
        },
        move |result: Result<SessionId, String>| Message::SessionCreated {
            thread_id,
            session_id: result.as_ref().copied().unwrap_or(Uuid::nil()),
            result: result.map(|_| ()),
        },
    )
}

fn handle_stop(app: &mut App, thread_id: uuid::Uuid) {
    let state = &mut app.state;

    if let Some(session_id) = state.thread_sessions.remove(&thread_id) {
        state.session_states.remove(&session_id);
        state
            .pending_approvals
            .retain(|a| a.session_id != session_id);
        state.streaming_threads.remove(&thread_id);

        let receivers = app.event_receivers.clone();
        let mgr = app.session_manager.clone();
        tokio::spawn(async move {
            receivers.remove(&session_id).await;
            let mut mgr = mgr.lock().await;
            if let Err(e) = mgr.stop_session(session_id).await {
                tracing::error!("Failed to stop session: {e:#}");
            }
        });
    }
}

fn handle_approval(
    app: &mut App,
    session_id: SessionId,
    request_id: &str,
    approve: bool,
) {
    let mgr = app.session_manager.clone();
    let rid = request_id.to_string();
    tokio::spawn(async move {
        let mgr = mgr.lock().await;
        if let Err(e) = mgr.respond_to_approval(session_id, &rid, approve) {
            tracing::error!("Failed to respond to approval: {e:#}");
        }
    });
    app.state
        .pending_approvals
        .retain(|a| a.request_id != request_id);
}
