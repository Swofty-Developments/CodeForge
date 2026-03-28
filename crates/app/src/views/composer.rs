use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Border, Element, Length};

use codeforge_session::Provider;

use crate::message::{ComposerMessage, Message};
use crate::state::{AppState, SessionState};
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    if state.active_tab.is_none() {
        return Space::new().height(0).into();
    }

    let thread_id = state.active_tab.unwrap();
    let is_generating = state.is_thread_generating(thread_id);

    let status_pill = status_pill_view(state, thread_id);
    let provider_btn = provider_button(state);

    let input = text_input("Message...", &state.composer_text)
        .on_input(|s| Message::Composer(ComposerMessage::TextChanged(s)))
        .on_submit(Message::Composer(ComposerMessage::Send))
        .padding([10, 14])
        .size(14)
        .width(Length::Fill);

    let send_btn = send_button(is_generating);
    let input_row = row![input, send_btn]
        .spacing(8)
        .align_y(iced::Alignment::Center);

    let folder_btn = folder_button(state);

    let bottom_bar = row![provider_btn, folder_btn, Space::new().width(Length::Fill), status_pill]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .padding([0, 4]);

    let composer_col = column![input_row, bottom_bar].spacing(6);

    let composer_card = container(composer_col)
        .padding([12, 14])
        .max_width(768)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_CARD)),
            border: Border {
                color: theme::BORDER,
                width: 1.0,
                radius: theme::RADIUS_XL.into(),
            },
            ..Default::default()
        });

    container(composer_card)
        .width(Length::Fill)
        .padding([8, 20])
        .center_x(Length::Fill)
        .into()
}

fn status_pill_view(state: &AppState, thread_id: uuid::Uuid) -> Element<'_, Message> {
    let (label, color) = match state.thread_session_state(thread_id) {
        Some(SessionState::Ready) => ("Ready", theme::GREEN),
        Some(SessionState::Generating) => ("Working", theme::SKY),
        Some(SessionState::Starting) => ("Connecting", theme::AMBER),
        Some(SessionState::Error) => ("Error", theme::RED),
        None => ("", theme::TEXT_TERTIARY),
    };
    if label.is_empty() {
        return Space::new().width(0).into();
    }
    container(
        row![
            text("\u{25CF}").size(6).color(color),
            text(label).size(10).color(color),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
    )
    .padding([3, 8])
    .style(move |_theme| container::Style {
        background: Some(iced::Background::Color(theme::BG_MUTED)),
        border: Border {
            radius: theme::RADIUS_PILL.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn provider_button(state: &AppState) -> Element<'_, Message> {
    let label = match state.selected_provider {
        Provider::ClaudeCode => "Claude Code",
        Provider::Codex => "Codex",
    };
    button(text(label).size(11).color(theme::TEXT_SECONDARY))
        .on_press(Message::Composer(ComposerMessage::ToggleProviderPicker))
        .padding([3, 8])
        .style(|_theme, status| button::Style {
            background: match status {
                button::Status::Hovered => Some(iced::Background::Color(theme::BG_ACCENT)),
                _ => Some(iced::Background::Color(theme::BG_MUTED)),
            },
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                color: theme::BORDER,
                width: 1.0,
            },
            ..Default::default()
        })
        .into()
}

fn folder_button(state: &AppState) -> Element<'_, Message> {
    let folder_label = state
        .active_tab
        .and_then(|tid| {
            state
                .projects
                .iter()
                .find(|p| p.threads.iter().any(|t| t.id == tid))
                .map(|p| {
                    if p.path == "." {
                        "No folder".to_string()
                    } else {
                        std::path::Path::new(&p.path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| p.path.clone())
                    }
                })
        })
        .unwrap_or_else(|| "No folder".to_string());

    button(text(folder_label).size(11).color(theme::TEXT_TERTIARY))
        .on_press(Message::Composer(ComposerMessage::PickFolder))
        .padding([3, 8])
        .style(|_theme, status| button::Style {
            background: match status {
                button::Status::Hovered => Some(iced::Background::Color(theme::BG_ACCENT)),
                _ => None,
            },
            border: Border {
                radius: theme::RADIUS_PILL.into(),
                color: theme::BORDER,
                width: 1.0,
            },
            ..Default::default()
        })
        .into()
}

pub fn provider_picker_overlay(state: &AppState) -> Element<'_, Message> {
    let picker = provider_picker_panel(state);

    let panel = container(picker)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            })),
            ..Default::default()
        });

    iced::widget::mouse_area(panel)
        .on_press(Message::Composer(ComposerMessage::ToggleProviderPicker))
        .into()
}

fn provider_picker_panel(state: &AppState) -> Element<'_, Message> {
    let option = |provider: Provider, label: &str| -> Element<'_, Message> {
        let is_selected = state.selected_provider == provider;
        let label = label.to_string();
        button(
            row![
                if is_selected {
                    text("\u{2713}").size(12).color(theme::PRIMARY)
                } else {
                    text("  ").size(12)
                },
                text(label).size(12).color(if is_selected {
                    theme::TEXT
                } else {
                    theme::TEXT_SECONDARY
                }),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::Composer(ComposerMessage::ProviderChanged(
            provider,
        )))
        .padding([5, 10])
        .width(Length::Fill)
        .style(|_theme, status| button::Style {
            background: match status {
                button::Status::Hovered => Some(iced::Background::Color(theme::BG_ACCENT)),
                _ => None,
            },
            border: Border::default(),
            ..Default::default()
        })
        .into()
    };

    container(
        column![
            text("Select Model").size(14).color(theme::TEXT),
            Space::new().height(8),
            option(Provider::ClaudeCode, "Claude Code"),
            option(Provider::Codex, "Codex"),
        ]
        .spacing(2),
    )
    .padding(16)
    .max_width(280)
    .style(|_theme| container::Style {
        background: Some(iced::Background::Color(theme::BG_CARD)),
        border: Border {
            color: theme::BORDER_STRONG,
            width: 1.0,
            radius: theme::RADIUS_LG.into(),
        },
        ..Default::default()
    })
    .into()
}

fn send_button(is_generating: bool) -> Element<'static, Message> {
    if is_generating {
        button(text("\u{25A0}").size(14).color(iced::Color::WHITE))
            .padding([8, 8])
            .style(|_theme, _status| button::Style {
                background: Some(iced::Background::Color(theme::RED)),
                text_color: iced::Color::WHITE,
                border: Border {
                    radius: theme::RADIUS_PILL.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    } else {
        button(text("\u{2191}").size(16).color(iced::Color::WHITE))
            .on_press(Message::Composer(ComposerMessage::Send))
            .padding([6, 10])
            .style(|_theme, _status| button::Style {
                background: Some(iced::Background::Color(theme::PRIMARY)),
                text_color: iced::Color::WHITE,
                border: Border {
                    radius: theme::RADIUS_PILL.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}
