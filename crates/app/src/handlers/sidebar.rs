use uuid::Uuid;

use crate::message::{Message, SidebarMessage};
use crate::state::{self, PendingPopup, Project, Thread, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH};
use crate::App;

pub fn handle(app: &mut App, msg: SidebarMessage) -> iced::Task<Message> {
    let state = &mut app.state;

    match msg {
        SidebarMessage::SelectThread(id) => {
            state.active_tab = Some(id);
            if !state.open_tabs.contains(&id) {
                state.open_tabs.push(id);
            }
        }
        SidebarMessage::NewThread => return handle_new_thread(app, None),
        SidebarMessage::NewThreadInProject(pid) => return handle_new_thread(app, Some(pid)),
        SidebarMessage::DeleteThread(id) => {
            handle_delete_thread(app, id);
        }
        SidebarMessage::ToggleSidebar => {
            state.sidebar_visible = !state.sidebar_visible;
        }
        SidebarMessage::StartRename(id) => {
            let title = state
                .find_thread(id)
                .map(|t| t.title.clone())
                .unwrap_or_default();
            state.renaming_thread = Some((id, title));
            state.context_menu = None;
        }
        SidebarMessage::RenameTextChanged(text) => {
            if let Some((_, ref mut buf)) = state.renaming_thread {
                *buf = text;
            }
        }
        SidebarMessage::ConfirmRename => {
            if let Some((id, ref new_title)) = state.renaming_thread {
                let new_title = new_title.clone();
                if !new_title.trim().is_empty() {
                    if let Some(thread) = state.find_thread_mut(id) {
                        thread.title = new_title.clone();
                    }
                    if let Some(ref db) = app.db {
                        if let Ok(db) = db.lock() {
                            let _ = codeforge_persistence::queries::update_thread_title(
                                db.conn(), id, &new_title,
                            );
                        }
                    }
                }
            }
            state.renaming_thread = None;
        }
        SidebarMessage::CancelRename => {
            state.renaming_thread = None;
        }
        SidebarMessage::ShowThreadContextMenu(id) => {
            state.context_menu = Some(state::ContextMenu::Thread(id));
        }
        SidebarMessage::ShowProjectContextMenu(id) => {
            state.context_menu = Some(state::ContextMenu::Project(id));
        }
        SidebarMessage::CloseContextMenu => {
            state.context_menu = None;
        }
        SidebarMessage::SetThreadColor(id, color) => {
            if let Some(thread) = state.find_thread_mut(id) {
                thread.color = color.clone();
            }
            if let Some(ref db) = app.db {
                if let Ok(db) = db.lock() {
                    let _ = codeforge_persistence::queries::update_thread_color(
                        db.conn(), id, color.as_deref(),
                    );
                }
            }
            state.context_menu = None;
        }
        SidebarMessage::RenameProject(id) => {
            let name = state
                .projects
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            state.renaming_project = Some((id, name));
            state.context_menu = None;
        }
        SidebarMessage::ProjectRenameTextChanged(text) => {
            if let Some((_, ref mut buf)) = state.renaming_project {
                *buf = text;
            }
        }
        SidebarMessage::ConfirmProjectRename => {
            if let Some((id, ref new_name)) = state.renaming_project {
                let new_name = new_name.clone();
                if !new_name.trim().is_empty() {
                    if let Some(project) = state.projects.iter_mut().find(|p| p.id == id) {
                        project.name = new_name;
                    }
                }
            }
            state.renaming_project = None;
        }
        SidebarMessage::CancelProjectRename => {
            state.renaming_project = None;
        }
        SidebarMessage::SetProjectColor(id, color) => {
            if let Some(project) = state.projects.iter_mut().find(|p| p.id == id) {
                project.color = color;
            }
            state.context_menu = None;
        }
        SidebarMessage::DeleteProject(id) => {
            state.pending_popup = Some(PendingPopup::ConfirmDeleteGroup { project_id: id });
            state.context_menu = None;
        }
        SidebarMessage::ToggleProjectCollapse(id) => {
            if let Some(project) = state.projects.iter_mut().find(|p| p.id == id) {
                project.collapsed = !project.collapsed;
            }
        }
        SidebarMessage::StartDragThread(id) => {
            // Only allow dragging uncategorized threads
            if state.is_uncategorized(id) {
                state.dragging_thread = Some(id);
            }
        }
        SidebarMessage::DropOnProject(project_id) => {
            if let Some(thread_id) = state.dragging_thread.take() {
                // Move thread from current project to target
                let mut thread = None;
                for project in &mut state.projects {
                    if let Some(idx) = project.threads.iter().position(|t| t.id == thread_id) {
                        thread = Some(project.threads.remove(idx));
                        break;
                    }
                }
                if let Some(thread) = thread {
                    if let Some(project) = state.projects.iter_mut().find(|p| p.id == project_id) {
                        project.threads.push(thread);
                    }
                }
            }
        }
        SidebarMessage::CancelDrag => {
            state.dragging_thread = None;
        }
        SidebarMessage::StartResize => {
            state.sidebar_dragging = true;
        }
        SidebarMessage::Resize(x) => {
            if state.sidebar_dragging {
                state.sidebar_width = x.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
            }
        }
        SidebarMessage::EndResize => {
            state.sidebar_dragging = false;
        }
    }

    iced::Task::none()
}

fn ensure_uncategorized(app: &mut App) -> usize {
    if let Some(idx) = app.state.projects.iter().position(|p| p.path == ".") {
        return idx;
    }
    let pid = Uuid::new_v4();
    let project = Project {
        id: pid,
        name: "Uncategorized".into(),
        path: ".".into(),
        color: None,
        collapsed: false,
        threads: Vec::new(),
    };
    if let Some(ref db) = app.db {
        if let Ok(db) = db.lock() {
            let db_project = codeforge_persistence::Project {
                id: pid,
                path: ".".to_string(),
                name: "Uncategorized".to_string(),
                created_at: chrono::Utc::now(),
            };
            let _ = codeforge_persistence::queries::insert_project(db.conn(), &db_project);
        }
    }
    app.state.projects.push(project);
    app.state.projects.len() - 1
}

fn handle_new_thread(app: &mut App, project_id: Option<Uuid>) -> iced::Task<Message> {
    let thread_id = Uuid::new_v4();
    let thread = Thread {
        id: thread_id,
        title: format!(
            "Thread {}",
            app.state.projects.iter().flat_map(|p| &p.threads).count() + 1
        ),
        color: None,
        provider: app.state.selected_provider,
        messages: Vec::new(),
        is_active: false,
    };

    let target_idx = match project_id {
        Some(pid) => app
            .state
            .projects
            .iter()
            .position(|p| p.id == pid)
            .unwrap_or_else(|| ensure_uncategorized(app)),
        None => ensure_uncategorized(app),
    };
    let target_project_id = app.state.projects[target_idx].id;

    if let Some(ref db) = app.db {
        if let Ok(db) = db.lock() {
            let now = chrono::Utc::now();
            let db_thread = codeforge_persistence::Thread {
                id: thread_id,
                project_id: target_project_id,
                title: thread.title.clone(),
                color: None,
                created_at: now,
                updated_at: now,
            };
            if let Err(e) = codeforge_persistence::queries::insert_thread(db.conn(), &db_thread) {
                tracing::error!("Failed to insert thread: {e:#}");
            }
        }
    }

    app.state.projects[target_idx].threads.push(thread);
    app.state.projects[target_idx].collapsed = false;
    app.state.open_tabs.push(thread_id);
    app.state.active_tab = Some(thread_id);

    iced::Task::none()
}

fn handle_delete_thread(app: &mut App, id: uuid::Uuid) {
    let state = &mut app.state;

    if let Some(session_id) = state.thread_sessions.remove(&id) {
        state.session_states.remove(&session_id);
        state
            .pending_approvals
            .retain(|a| a.session_id != session_id);
        state.streaming_threads.remove(&id);
        let receivers = app.event_receivers.clone();
        let mgr = app.session_manager.clone();
        tokio::spawn(async move {
            receivers.remove(&session_id).await;
            let mut mgr = mgr.lock().await;
            let _ = mgr.stop_session(session_id).await;
        });
    }

    if let Some(ref db) = app.db {
        if let Ok(db) = db.lock() {
            let _ = codeforge_persistence::queries::delete_messages_by_thread(db.conn(), id);
            let _ = codeforge_persistence::queries::delete_thread(db.conn(), id);
        }
    }

    for project in &mut state.projects {
        project.threads.retain(|t| t.id != id);
    }
    state.open_tabs.retain(|&t| t != id);
    if state.active_tab == Some(id) {
        state.active_tab = state.open_tabs.last().copied();
    }
}
