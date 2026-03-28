mod handlers;
mod message;
mod state;
mod subscriptions;
mod theme;
mod views;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use iced::widget::{column, container, mouse_area, row, Space};
use iced::{Element, Length, Theme};

use codeforge_persistence::Database;
use codeforge_session::SessionManager;
use message::{DbPayload, Message, SidebarMessage};
use state::AppState;
use subscriptions::agent::AgentEventReceivers;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("codeforge=debug,info")
        .init();

    tracing::info!("Starting CodeForge...");

    iced::application(App::new, handlers::update, view)
        .title("CodeForge")
        .theme(|_: &App| Theme::KanagawaDragon)
        .window_size((1200.0, 800.0))
        .centered()
        .subscription(|app: &App| {
            subscriptions::agent::agent_events(app.event_receivers.clone())
        })
        .run()
}

pub struct App {
    pub state: AppState,
    pub db: Option<Arc<Mutex<Database>>>,
    pub session_manager: Arc<tokio::sync::Mutex<SessionManager>>,
    pub event_receivers: AgentEventReceivers,
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
        let task = iced::Task::perform(load_db_data(), |result| {
            Message::DbLoaded(result.map_err(|e| format!("{e:#}")))
        });
        (app, task)
    }
}

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

pub fn dirs_db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".codeforge")
        .join("codeforge.db")
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

    let layout: Element<'_, Message> = if state.sidebar_visible {
        let drag_handle: Element<'_, Message> = mouse_area(
            container(Space::new().width(4).height(Length::Fill)).style(
                |_theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(theme::BORDER)),
                    ..Default::default()
                },
            ),
        )
        .on_press(Message::Sidebar(SidebarMessage::StartResize))
        .interaction(iced::mouse::Interaction::ResizingHorizontally)
        .into();

        row![views::sidebar::view(state), drag_handle, main_panel]
            .height(Length::Fill)
            .into()
    } else {
        container(main_panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    let layout: Element<'_, Message> = if state.sidebar_dragging {
        mouse_area(layout)
            .on_move(|p| Message::Sidebar(SidebarMessage::Resize(p.x)))
            .on_release(Message::Sidebar(SidebarMessage::EndResize))
            .interaction(iced::mouse::Interaction::ResizingHorizontally)
            .into()
    } else {
        layout
    };

    let layout: Element<'_, Message> = if state.context_menu.is_some() {
        mouse_area(layout)
            .on_press(Message::Sidebar(SidebarMessage::CloseContextMenu))
            .into()
    } else if state.dragging_thread.is_some() {
        mouse_area(layout)
            .on_release(Message::Sidebar(SidebarMessage::CancelDrag))
            .into()
    } else {
        layout
    };

    // Layer overlays
    let mut layers: Vec<Element<'_, Message>> = vec![layout];
    if state.settings_open {
        layers.push(views::settings::view(state));
    }
    if state.provider_picker_open {
        layers.push(views::composer::provider_picker_overlay(state));
    }
    if let Some(popup) = views::popup::view(state) {
        layers.push(popup);
    }

    if layers.len() == 1 {
        layers.remove(0)
    } else {
        iced::widget::Stack::from_vec(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
