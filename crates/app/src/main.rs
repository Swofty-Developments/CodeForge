mod message;
mod state;
mod theme;
mod views;

use iced::widget::{column, container, row, stack};
use iced::{Element, Length, Theme};
use uuid::Uuid;

fn theme_fn(_: &App) -> Theme {
    Theme::CatppuccinMocha
}

use message::{ComposerMessage, Message, SettingsMessage, SidebarMessage, TabMessage};
use state::{AppState, ChatMessage, MessageRole, Project, Thread};

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
        .run()
}

struct App {
    state: AppState,
}

impl App {
    fn new() -> (Self, iced::Task<Message>) {
        (
            Self {
                state: AppState::default(),
            },
            iced::Task::none(),
        )
    }
}

fn update(app: &mut App, message: Message) -> iced::Task<Message> {
    let state = &mut app.state;

    match message {
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
                    title: format!("Thread {}", state.projects.iter().flat_map(|p| &p.threads).count() + 1),
                    provider: state.selected_provider,
                    messages: Vec::new(),
                    is_active: false,
                };

                if state.projects.is_empty() {
                    state.projects.push(Project {
                        id: Uuid::new_v4(),
                        name: "Default Project".into(),
                        path: ".".into(),
                        threads: Vec::new(),
                    });
                }

                state.projects[0].threads.push(thread);
                state.open_tabs.push(thread_id);
                state.active_tab = Some(thread_id);
            }
            SidebarMessage::DeleteThread(id) => {
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
        Message::Chat(_msg) => {}
        Message::Composer(msg) => match msg {
            ComposerMessage::TextChanged(text) => {
                state.composer_text = text;
            }
            ComposerMessage::Send => {
                if state.composer_text.trim().is_empty() {
                    return iced::Task::none();
                }
                let content = state.composer_text.clone();
                state.composer_text.clear();

                if let Some(thread) = state.active_thread_mut() {
                    thread.messages.push(ChatMessage {
                        id: Uuid::new_v4(),
                        role: MessageRole::User,
                        content: content.clone(),
                    });
                    // TODO: Send to agent subprocess in Phase 3
                    thread.messages.push(ChatMessage {
                        id: Uuid::new_v4(),
                        role: MessageRole::Assistant,
                        content: format!("[Agent not connected yet] Echo: {content}"),
                    });
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
            }
        },
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
