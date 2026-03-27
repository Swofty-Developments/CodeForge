mod message;
mod state;
mod subscriptions;
mod theme;
mod views;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use iced::widget::{column, container, row, stack};
use iced::{Element, Length, Subscription, Theme};
use uuid::Uuid;

fn theme_fn(_: &App) -> Theme {
    Theme::KanagawaDragon
}

use codeforge_persistence::Database;
use codeforge_session::{AgentEvent, SessionId, SessionManager};
use message::{
    AgentMessage, ChatMessage, ComposerMessage, DbPayload, Message, SettingsMessage,
    SidebarMessage, TabMessage,
};
use state::{AppState, MessageRole, PendingApproval, Project, SessionState, Thread};
use subscriptions::agent::AgentEventReceivers;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("codeforge=debug,info")
        .init();

    tracing::info!("Starting CodeForge...");

    iced::application(App::new, update, view)
        .title("CodeForge")
        .theme(theme_fn)
        .window_size((1200.0, 800.0))
        .centered()
        .subscription(subscription)
        .run()
}

struct App {
    state: AppState,
    db: Option<Arc<Mutex<Database>>>,
    session_manager: Arc<tokio::sync::Mutex<SessionManager>>,
    event_receivers: AgentEventReceivers,
}

impl App {
    fn new() -> (Self, iced::Task<Message>) {
        let event_receivers = AgentEventReceivers::new();

        let app = Self {
            state: AppState::default(),
            db: None,
            session_manager: Arc::new(tokio::sync::Mutex::new(SessionManager::new())),
            event_receivers,
        };

        // Async task to open DB and load data
        let task = iced::Task::perform(load_db_data(), |result| {
            Message::DbLoaded(result.map_err(|e| format!("{e:#}")))
        });

        (app, task)
    }
}

/// Load database and all projects/threads/messages
async fn load_db_data() -> anyhow::Result<DbPayload> {
    let db_dir = dirs_db_path();
    if let Some(parent) = db_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let db = Database::open(&db_dir)?;
    let conn = db.conn();

    let projects = codeforge_persistence::queries::get_all_projects(conn)?;

    let mut threads_by_project = Vec::new();
    for project in &projects {
        let threads = codeforge_persistence::queries::get_threads_by_project(conn, project.id)?;
        threads_by_project.push((project.id, threads));
    }

    let mut messages_by_thread = Vec::new();
    for (_, threads) in &threads_by_project {
        for thread in threads {
            let messages =
                codeforge_persistence::queries::get_messages_by_thread(conn, thread.id)?;
            messages_by_thread.push((thread.id, messages));
        }
    }

    Ok(DbPayload {
        projects,
        threads_by_project,
        messages_by_thread,
    })
}

fn dirs_db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".codeforge")
        .join("codeforge.db")
}

fn subscription(app: &App) -> Subscription<Message> {
    subscriptions::agent::agent_events(app.event_receivers.clone())
}

fn update(app: &mut App, message: Message) -> iced::Task<Message> {
    let state = &mut app.state;

    match message {
        Message::DbLoaded(result) => {
            match result {
                Ok(payload) => {
                    // Open DB handle for ongoing writes
                    let db_path = dirs_db_path();
                    match Database::open(&db_path) {
                        Ok(db) => {
                            app.db = Some(Arc::new(Mutex::new(db)));
                        }
                        Err(e) => {
                            tracing::error!("Failed to open database: {e:#}");
                        }
                    }

                    // Build messages lookup
                    let mut messages_map: std::collections::HashMap<
                        Uuid,
                        Vec<codeforge_persistence::Message>,
                    > = std::collections::HashMap::new();
                    for (thread_id, msgs) in payload.messages_by_thread {
                        messages_map.insert(thread_id, msgs);
                    }

                    // Convert persistence models into app state models
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
                                                    r if *r == codeforge_persistence::MessageRole::User => MessageRole::User,
                                                    r if *r == codeforge_persistence::MessageRole::Assistant => MessageRole::Assistant,
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
                            threads,
                        });
                    }

                    // Load settings
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
                Err(e) => {
                    tracing::error!("Failed to load database: {e}");
                    state.db_loaded = true; // still mark as loaded so UI works
                }
            }
        }
        Message::Sidebar(msg) => match msg {
            SidebarMessage::SelectThread(id) => {
                state.active_tab = Some(id);
                if !state.open_tabs.contains(&id) {
                    state.open_tabs.push(id);
                }
            }
            SidebarMessage::NewThread => {
                let thread_id = Uuid::new_v4();
                let thread = Thread {
                    id: thread_id,
                    title: format!(
                        "Thread {}",
                        state
                            .projects
                            .iter()
                            .flat_map(|p| &p.threads)
                            .count()
                            + 1
                    ),
                    provider: state.selected_provider,
                    messages: Vec::new(),
                    is_active: false,
                };

                if state.projects.is_empty() {
                    let project_id = Uuid::new_v4();
                    let project = Project {
                        id: project_id,
                        name: "Default Project".into(),
                        path: ".".into(),
                        threads: Vec::new(),
                    };

                    // Persist project to DB
                    if let Some(ref db) = app.db {
                        if let Ok(db) = db.lock() {
                            let db_project = codeforge_persistence::Project {
                                id: project_id,
                                path: ".".to_string(),
                                name: "Default Project".to_string(),
                                created_at: chrono::Utc::now(),
                            };
                            if let Err(e) =
                                codeforge_persistence::queries::insert_project(db.conn(), &db_project)
                            {
                                tracing::error!("Failed to insert project: {e:#}");
                            }
                        }
                    }

                    state.projects.push(project);
                }

                let project_id = state.projects[0].id;

                // Persist thread to DB
                if let Some(ref db) = app.db {
                    if let Ok(db) = db.lock() {
                        let now = chrono::Utc::now();
                        let db_thread = codeforge_persistence::Thread {
                            id: thread_id,
                            project_id,
                            title: thread.title.clone(),
                            created_at: now,
                            updated_at: now,
                        };
                        if let Err(e) =
                            codeforge_persistence::queries::insert_thread(db.conn(), &db_thread)
                        {
                            tracing::error!("Failed to insert thread: {e:#}");
                        }
                    }
                }

                state.projects[0].threads.push(thread);
                state.open_tabs.push(thread_id);
                state.active_tab = Some(thread_id);
            }
            SidebarMessage::DeleteThread(id) => {
                // Stop session if active
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

                // Delete from DB
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
            SidebarMessage::ToggleSidebar => {
                state.sidebar_visible = !state.sidebar_visible;
            }
        },
        Message::Chat(msg) => match msg {
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
                        state.pending_approvals.retain(|a| a.request_id != request_id);
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
                        state.pending_approvals.retain(|a| a.request_id != request_id);
                    }
                }
            }
        },
        Message::Composer(msg) => match msg {
            ComposerMessage::TextChanged(text) => {
                state.composer_text = text;
            }
            ComposerMessage::Send => {
                if state.composer_text.trim().is_empty() {
                    return iced::Task::none();
                }

                let Some(thread_id) = state.active_tab else {
                    return iced::Task::none();
                };

                // Don't send if already generating
                if state.is_thread_generating(thread_id) {
                    return iced::Task::none();
                }

                let content = state.composer_text.clone();
                state.composer_text.clear();

                // Add user message to UI
                let user_msg_id = Uuid::new_v4();
                if let Some(thread) = state.find_thread_mut(thread_id) {
                    thread.messages.push(state::ChatMessage {
                        id: user_msg_id,
                        role: MessageRole::User,
                        content: content.clone(),
                    });
                }

                // Persist user message to DB
                if let Some(ref db) = app.db {
                    if let Ok(db) = db.lock() {
                        // Find the project_id for this thread
                        let project_id = state
                            .projects
                            .iter()
                            .find(|p| p.threads.iter().any(|t| t.id == thread_id))
                            .map(|p| p.id);
                        if let Some(_project_id) = project_id {
                            let db_msg = codeforge_persistence::Message {
                                id: user_msg_id,
                                thread_id,
                                role: codeforge_persistence::MessageRole::User,
                                content: content.clone(),
                                created_at: chrono::Utc::now(),
                            };
                            if let Err(e) =
                                codeforge_persistence::queries::insert_message(db.conn(), &db_msg)
                            {
                                tracing::error!("Failed to persist user message: {e:#}");
                            }
                        }
                    }
                }

                // Check if session exists for this thread
                if let Some(&session_id) = state.thread_sessions.get(&thread_id) {
                    // Session exists, send message
                    state
                        .session_states
                        .insert(session_id, SessionState::Generating);
                    let mgr = app.session_manager.clone();
                    return iced::Task::perform(
                        async move {
                            let mut mgr = mgr.lock().await;
                            mgr.send_message(session_id, &content)
                                .await
                                .map_err(|e| format!("{e:#}"))
                        },
                        move |result| Message::MessageSent {
                            thread_id,
                            result,
                        },
                    );
                } else {
                    // No session, create one first then send
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

                    state.streaming_threads.remove(&thread_id);

                    return iced::Task::perform(
                        async move {
                            let cwd_path = std::path::PathBuf::from(&cwd);
                            let mut mgr = mgr.lock().await;
                            match mgr.create_session(provider, &cwd_path).await {
                                Ok((session_id, event_rx)) => {
                                    receivers.insert(session_id, event_rx).await;
                                    // Now send the message
                                    if let Err(e) =
                                        mgr.send_message(session_id, &content).await
                                    {
                                        tracing::error!(
                                            "Failed to send initial message: {e:#}"
                                        );
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
                    );
                }
            }
            ComposerMessage::ProviderChanged(provider) => {
                state.selected_provider = provider;
            }
        },
        Message::Tab(msg) => match msg {
            TabMessage::Select(id) => {
                state.active_tab = Some(id);
            }
            TabMessage::Close(id) => {
                state.open_tabs.retain(|&t| t != id);
                if state.active_tab == Some(id) {
                    state.active_tab = state.open_tabs.last().copied();
                }
            }
        },
        Message::Settings(msg) => match msg {
            SettingsMessage::Open => state.settings_open = true,
            SettingsMessage::Close => state.settings_open = false,
            SettingsMessage::ApprovalModeChanged(mode) => {
                state.approval_mode = mode;
                // Persist
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
        },
        Message::Agent(agent_msg) => match agent_msg {
            AgentMessage::Event { session_id, event } => {
                // Find which thread this session belongs to
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
                        // Append to streaming message or create one
                        if let Some(&msg_id) = state.streaming_threads.get(&thread_id) {
                            // Append to existing streaming message
                            if let Some(thread) = state.find_thread_mut(thread_id) {
                                if let Some(msg) =
                                    thread.messages.iter_mut().find(|m| m.id == msg_id)
                                {
                                    msg.content.push_str(&text);
                                }
                            }
                        } else {
                            // Create new streaming message
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

                        // Finalize streaming message - persist to DB
                        if let Some(msg_id) = state.streaming_threads.remove(&thread_id) {
                            if let Some(thread) = state.find_thread(thread_id) {
                                if let Some(msg) =
                                    thread.messages.iter().find(|m| m.id == msg_id)
                                {
                                    if let Some(ref db) = app.db {
                                        if let Ok(db) = db.lock() {
                                            let db_msg = codeforge_persistence::Message {
                                                id: msg_id,
                                                thread_id,
                                                role:
                                                    codeforge_persistence::MessageRole::Assistant,
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

                        // Remove streaming message and add error
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
            }
            AgentMessage::StartSession { thread_id } => {
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

                return iced::Task::perform(
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
                );
            }
            AgentMessage::StopSession { thread_id } => {
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
            AgentMessage::ApprovalResponse {
                session_id,
                request_id,
                approve,
            } => {
                let mgr = app.session_manager.clone();
                let rid = request_id.clone();
                tokio::spawn(async move {
                    let mgr = mgr.lock().await;
                    if let Err(e) = mgr.respond_to_approval(session_id, &rid, approve) {
                        tracing::error!("Failed to respond to approval: {e:#}");
                    }
                });
                state
                    .pending_approvals
                    .retain(|a| a.request_id != request_id);
            }
        },
        Message::SessionCreated {
            thread_id,
            session_id,
            result,
        } => match result {
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
        },
        Message::MessageSent { thread_id, result } => {
            if let Err(e) = result {
                tracing::error!("Failed to send message to session: {e}");
                if let Some(thread) = state.find_thread_mut(thread_id) {
                    thread.messages.push(state::ChatMessage {
                        id: Uuid::new_v4(),
                        role: MessageRole::System,
                        content: format!("Failed to send message: {e}"),
                    });
                }
            }
        }
    }

    iced::Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    let state = &app.state;

    let main_content = column![
        views::tabs::view(state),
        views::chat::view(state),
        views::composer::view(state),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    let main_panel = container(main_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::BG_BASE)),
            ..Default::default()
        });

    let layout = if state.sidebar_visible {
        row![views::sidebar::view(state), main_panel]
            .height(Length::Fill)
            .into()
    } else {
        container(main_panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    if state.settings_open {
        stack![layout, views::settings::view(state)].into()
    } else {
        layout
    }
}
