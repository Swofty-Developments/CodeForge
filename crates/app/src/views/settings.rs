use iced::widget::{button, column, container, text};
use iced::{Element, Length};

use crate::message::{Message, SettingsMessage};
use crate::state::AppState;
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = text("Settings").size(20).color(theme::TEXT);

    let close_btn = button(text("Close").size(13).color(theme::SUBTEXT))
        .on_press(Message::Settings(SettingsMessage::Close))
        .padding([6, 12]);

    let provider_section = column![
        text("Default Provider").size(14).color(theme::TEXT),
        text(format!("{}", state.selected_provider))
            .size(13)
            .color(theme::SUBTEXT),
    ]
    .spacing(4);

    let approval_section = column![
        text("Approval Mode").size(14).color(theme::TEXT),
        text(format!("{}", state.approval_mode))
            .size(13)
            .color(theme::SUBTEXT),
    ]
    .spacing(4);

    let content = column![
        header,
        close_btn,
        provider_section,
        approval_section,
    ]
    .spacing(16)
    .padding(24)
    .width(400);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color {
                a: 0.85,
                ..theme::BG_BASE
            })),
            ..Default::default()
        })
        .into()
}
