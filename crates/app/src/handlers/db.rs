use std::sync::{Arc, Mutex};
use uuid::Uuid;

use codeforge_persistence::Database;

use crate::message::{DbPayload, Message};
use crate::state::{self, MessageRole, Project, Thread};
use crate::{dirs_db_path, App};

pub fn handle(app: &mut App, result: Result<DbPayload, String>) -> iced::Task<Message> {
    match result {
        Ok(payload) => load_payload(app, payload),
        Err(e) => {
            tracing::error!("Failed to load database: {e}");
            app.state.db_loaded = true;
        }
    }
    iced::Task::none()
}

fn load_payload(app: &mut App, payload: DbPayload) {
    let state = &mut app.state;

    let db_path = dirs_db_path();
    match Database::open(&db_path) {
        Ok(db) => app.db = Some(Arc::new(Mutex::new(db))),
        Err(e) => tracing::error!("Failed to open database: {e:#}"),
    }

    let mut messages_map: std::collections::HashMap<Uuid, Vec<codeforge_persistence::Message>> =
        std::collections::HashMap::new();
    for (thread_id, msgs) in payload.messages_by_thread {
        messages_map.insert(thread_id, msgs);
    }

    for project in &payload.projects {
        let threads_for_project = payload
            .threads_by_project
            .iter()
            .find(|(pid, _)| *pid == project.id)
            .map(|(_, threads)| threads.as_slice())
            .unwrap_or(&[]);

        let threads: Vec<Thread> = threads_for_project
            .iter()
            .map(|t| {
                let msgs = messages_map
                    .get(&t.id)
                    .map(|ms| {
                        ms.iter()
                            .map(|m| state::ChatMessage {
                                id: m.id,
                                role: match &m.role {
                                    r if *r == codeforge_persistence::MessageRole::User => {
                                        MessageRole::User
                                    }
                                    r if *r == codeforge_persistence::MessageRole::Assistant => {
                                        MessageRole::Assistant
                                    }
                                    _ => MessageRole::System,
                                },
                                content: m.content.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Thread {
                    id: t.id,
                    title: t.title.clone(),
                    color: t.color.clone(),
                    provider: codeforge_session::Provider::ClaudeCode,
                    messages: msgs,
                    is_active: false,
                }
            })
            .collect();

        state.projects.push(Project {
            id: project.id,
            name: project.name.clone(),
            path: project.path.clone(),
            color: None,
            collapsed: false,
            threads,
        });
    }

    if let Some(ref db) = app.db {
        if let Ok(db) = db.lock() {
            if let Ok(Some(path)) =
                codeforge_persistence::queries::get_setting(db.conn(), "claude_path")
            {
                state.claude_path = path;
            }
            if let Ok(Some(path)) =
                codeforge_persistence::queries::get_setting(db.conn(), "codex_path")
            {
                state.codex_path = path;
            }
            if let Ok(Some(mode)) =
                codeforge_persistence::queries::get_setting(db.conn(), "approval_mode")
            {
                state.approval_mode = match mode.as_str() {
                    "auto_approve" => state::ApprovalMode::AutoApprove,
                    _ => state::ApprovalMode::Supervised,
                };
            }
        }
    }

    state.db_loaded = true;
    tracing::info!("Database loaded: {} projects", state.projects.len());
}
