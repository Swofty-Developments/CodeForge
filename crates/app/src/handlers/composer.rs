use uuid::Uuid;

use codeforge_session::SessionId;

use crate::message::{ComposerMessage, Message};
use crate::state::{self, MessageRole, SessionState};
use crate::App;

pub fn handle(app: &mut App, msg: ComposerMessage) -> iced::Task<Message> {
    match msg {
        ComposerMessage::TextChanged(text) => {
            app.state.composer_text = text;
        }
        ComposerMessage::Send => {
            return handle_send(app);
        }
        ComposerMessage::ProviderChanged(provider) => {
            app.state.selected_provider = provider;
            app.state.provider_picker_open = false;
        }
        ComposerMessage::ToggleProviderPicker => {
            app.state.provider_picker_open = !app.state.provider_picker_open;
        }
        ComposerMessage::PickFolder => {
            return iced::Task::perform(
                async {
                    let handle = rfd::AsyncFileDialog::new()
                        .set_title("Select project folder")
                        .pick_folder()
                        .await;
                    handle.map(|h| h.path().to_string_lossy().to_string())
                },
                |path| Message::Composer(ComposerMessage::FolderPicked(path)),
            );
        }
        ComposerMessage::FolderPicked(Some(path)) => {
            let existing = app.state.projects.iter().find(|p| p.path == path);
            if let Some(project) = existing {
                // Group exists - move active thread there
                move_active_thread_to_project(app, project.id);
            } else {
                // Ask user to create a new group
                app.state.pending_popup =
                    Some(state::PendingPopup::ConfirmNewGroup { directory: path });
            }
        }
        ComposerMessage::FolderPicked(None) => {}
        ComposerMessage::ConfirmNewGroup(path) => {
            let dir_name = std::path::Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());

            let project_id = Uuid::new_v4();
            let project = state::Project {
                id: project_id,
                name: dir_name.clone(),
                path: path.clone(),
                color: None,
                collapsed: false,
                threads: Vec::new(),
            };
            if let Some(ref db) = app.db {
                if let Ok(db) = db.lock() {
                    let db_project = codeforge_persistence::Project {
                        id: project_id,
                        path,
                        name: dir_name,
                        created_at: chrono::Utc::now(),
                    };
                    let _ =
                        codeforge_persistence::queries::insert_project(db.conn(), &db_project);
                }
            }
            app.state.projects.push(project);
            app.state.pending_popup = None;

            // Move active thread to the new group
            move_active_thread_to_project(app, project_id);
        }
    }
    iced::Task::none()
}

fn handle_send(app: &mut App) -> iced::Task<Message> {
    let state = &mut app.state;

    if state.composer_text.trim().is_empty() {
        return iced::Task::none();
    }
    let Some(thread_id) = state.active_tab else {
        return iced::Task::none();
    };
    if state.is_thread_generating(thread_id) {
        return iced::Task::none();
    }

    let content = state.composer_text.clone();
    state.composer_text.clear();

    let user_msg_id = Uuid::new_v4();
    if let Some(thread) = state.find_thread_mut(thread_id) {
        thread.messages.push(state::ChatMessage {
            id: user_msg_id,
            role: MessageRole::User,
            content: content.clone(),
        });
    }

    if let Some(ref db) = app.db {
        if let Ok(db) = db.lock() {
            let has_project = state
                .projects
                .iter()
                .any(|p| p.threads.iter().any(|t| t.id == thread_id));
            if has_project {
                let db_msg = codeforge_persistence::Message {
                    id: user_msg_id,
                    thread_id,
                    role: codeforge_persistence::MessageRole::User,
                    content: content.clone(),
                    created_at: chrono::Utc::now(),
                };
                if let Err(e) = codeforge_persistence::queries::insert_message(db.conn(), &db_msg)
                {
                    tracing::error!("Failed to persist user message: {e:#}");
                }
            }
        }
    }

    if let Some(&session_id) = state.thread_sessions.get(&thread_id) {
        state
            .session_states
            .insert(session_id, SessionState::Generating);
        let mgr = app.session_manager.clone();
        iced::Task::perform(
            async move {
                let mut mgr = mgr.lock().await;
                mgr.send_message(session_id, &content)
                    .await
                    .map_err(|e| format!("{e:#}"))
            },
            move |result| Message::MessageSent { thread_id, result },
        )
    } else {
        let provider = state
            .find_thread(thread_id)
            .map(|t| t.provider)
            .unwrap_or(state.selected_provider);
        let mgr = app.session_manager.clone();
        let receivers = app.event_receivers.clone();
        let cwd = state
            .projects
            .iter()
            .find(|p| p.threads.iter().any(|t| t.id == thread_id))
            .map(|p| p.path.clone())
            .unwrap_or_else(|| ".".to_string());

        state.streaming_threads.remove(&thread_id);

        iced::Task::perform(
            async move {
                let cwd_path = std::path::PathBuf::from(&cwd);
                let mut mgr = mgr.lock().await;
                match mgr.create_session(provider, &cwd_path).await {
                    Ok((session_id, event_rx)) => {
                        receivers.insert(session_id, event_rx).await;
                        if let Err(e) = mgr.send_message(session_id, &content).await {
                            tracing::error!("Failed to send initial message: {e:#}");
                        }
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
}

fn move_active_thread_to_project(app: &mut App, target_project_id: Uuid) {
    let Some(thread_id) = app.state.active_tab else {
        return;
    };

    // Find and remove the thread from its current project
    let mut thread = None;
    for project in &mut app.state.projects {
        if let Some(idx) = project.threads.iter().position(|t| t.id == thread_id) {
            thread = Some(project.threads.remove(idx));
            break;
        }
    }

    // Add to target project
    if let Some(thread) = thread {
        if let Some(project) = app
            .state
            .projects
            .iter_mut()
            .find(|p| p.id == target_project_id)
        {
            project.threads.push(thread);
        }
    }
}
