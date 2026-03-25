use iced::widget::{center, column, container, scrollable, text, Column};
use iced::{Element, Length};

use crate::message::Message;
use crate::state::{AppState, MessageRole};
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let thread = match state.active_thread() {
        Some(t) => t,
        None => {
            return center(
                text("Select a thread or start a new one")
                    .size(16)
                    .color(theme::SUBTEXT),
            )
            .into();
        }
    };

    if thread.messages.is_empty() {
        return center(
            column![
                text("Start a conversation")
                    .size(18)
                    .color(theme::SUBTEXT),
                text(format!("Using {}", thread.provider))
                    .size(13)
                    .color(theme::BG_OVERLAY),
            ]
            .spacing(8)
            .align_x(iced::Alignment::Center),
        )
        .into();
    }

    let messages: Vec<Element<'_, Message>> = thread
        .messages
        .iter()
        .map(|msg| {
            let (role_label, role_color) = match msg.role {
                MessageRole::User => ("You", theme::PRIMARY),
                MessageRole::Assistant => ("Agent", theme::GREEN),
                MessageRole::System => ("System", theme::PEACH),
            };

            let msg_widget = column![
                text(role_label).size(12).color(role_color),
                text(&msg.content).size(14).color(theme::TEXT),
            ]
            .spacing(4);

            container(msg_widget)
                .padding([8, 12])
                .width(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(theme::BG_SURFACE)),
                    border: iced::Border {
                        color: theme::BORDER,
                        width: 0.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                })
                .into()
        })
        .collect();

    let messages_col = Column::from_vec(messages).spacing(8).padding(12);

    scrollable(messages_col)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
