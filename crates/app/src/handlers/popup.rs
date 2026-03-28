use crate::message::{Message, PopupMessage};
use crate::state::PendingPopup;
use crate::App;

pub fn handle(app: &mut App, msg: PopupMessage) -> iced::Task<Message> {
    match msg {
        PopupMessage::ConfirmDeleteGroup { delete_threads } => {
            if let Some(PendingPopup::ConfirmDeleteGroup { project_id }) =
                app.state.pending_popup.take()
            {
                if delete_threads {
                    // Delete all threads in the project
                    if let Some(project) = app.state.projects.iter().find(|p| p.id == project_id) {
                        let thread_ids: Vec<_> = project.threads.iter().map(|t| t.id).collect();
                        for tid in &thread_ids {
                            app.state.open_tabs.retain(|&t| t != *tid);
                            if app.state.active_tab == Some(*tid) {
                                app.state.active_tab = None;
                            }
                        }
                        if let Some(ref db) = app.db {
                            if let Ok(db) = db.lock() {
                                for tid in &thread_ids {
                                    let _ = codeforge_persistence::queries::delete_messages_by_thread(db.conn(), *tid);
                                    let _ = codeforge_persistence::queries::delete_thread(db.conn(), *tid);
                                }
                                let _ = codeforge_persistence::queries::delete_project(db.conn(), project_id);
                            }
                        }
                    }
                    app.state.projects.retain(|p| p.id != project_id);
                } else {
                    // Move threads to "Uncategorized" or first other project
                    let threads = app
                        .state
                        .projects
                        .iter()
                        .find(|p| p.id == project_id)
                        .map(|p| p.threads.clone())
                        .unwrap_or_default();

                    if let Some(ref db) = app.db {
                        if let Ok(db) = db.lock() {
                            let _ = codeforge_persistence::queries::delete_project(db.conn(), project_id);
                        }
                    }

                    app.state.projects.retain(|p| p.id != project_id);

                    if !threads.is_empty() {
                        // Find or create "Uncategorized"
                        let uncategorized = app
                            .state
                            .projects
                            .iter()
                            .position(|p| p.name == "Uncategorized");
                        match uncategorized {
                            Some(idx) => {
                                app.state.projects[idx].threads.extend(threads);
                            }
                            None => {
                                let pid = uuid::Uuid::new_v4();
                                app.state.projects.push(crate::state::Project {
                                    id: pid,
                                    name: "Uncategorized".into(),
                                    path: ".".into(),
                                    color: None,
                                    collapsed: false,
                                    threads,
                                });
                            }
                        }
                    }
                }
            }
        }
        PopupMessage::CancelPopup => {
            app.state.pending_popup = None;
        }
    }

    iced::Task::none()
}
