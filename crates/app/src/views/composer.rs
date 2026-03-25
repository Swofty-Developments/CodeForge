use iced::widget::{button, container, row, text, text_input};
use iced::{Element, Length};

use crate::message::{ComposerMessage, Message};
use crate::state::{AppState, SessionState};
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    if state.active_tab.is_none() {
        return container(text("")).height(0).into();
    }

    let thread_id = state.active_tab.unwrap();
    let is_generating = state.is_thread_generating(thread_id);

    let input = text_input("Type a message...", &state.composer_text)
        .on_input(|s| Message::Composer(ComposerMessage::TextChanged(s)))
        .on_submit(Message::Composer(ComposerMessage::Send))
        .padding(10)
        .size(14)
        .width(Length::Fill);

    let send_btn = if is_generating {
        button(text("Generating...").size(14).color(theme::SUBTEXT)).padding([10, 20])
    } else {
        button(text("Send").size(14).color(theme::TEXT))
            .on_press(Message::Composer(ComposerMessage::Send))
            .padding([10, 20])
    };

    let session_status = match state.thread_session_state(thread_id) {
        Some(SessionState::Ready) => text("Connected").size(11).color(theme::GREEN),
        Some(SessionState::Generating) => text("Generating").size(11).color(theme::PEACH),
        Some(SessionState::Starting) => text("Connecting").size(11).color(theme::PEACH),
        Some(SessionState::Error) => text("Error").size(11).color(theme::RED),
        None => text("No session").size(11).color(theme::SUBTEXT),
    };

    let provider_label = text(format!("{}", state.selected_provider))
        .size(12)
        .color(theme::SUBTEXT);

    let content = row![provider_label, session_status, input, send_btn]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding(8);

    container(content)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_SURFACE)),
            border: iced::Border {
                color: theme::BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}
