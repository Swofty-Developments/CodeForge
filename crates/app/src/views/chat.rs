use iced::widget::{button, center, column, container, row, scrollable, text, Column};
use iced::{Element, Length};

use crate::message::{ChatMessage, Message};
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

    let thread_id = thread.id;
    let is_generating = state.is_thread_generating(thread_id);

    if thread.messages.is_empty() && !is_generating {
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

    let mut elements: Vec<Element<'_, Message>> = thread
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

    // Show streaming indicator
    if is_generating {
        let indicator = container(
            text("Agent is generating...")
                .size(12)
                .color(theme::PEACH),
        )
        .padding([4, 12])
        .width(Length::Fill);
        elements.push(indicator.into());
    }

    // Show pending approval prompts
    let approvals = state.active_thread_approvals();
    for approval in approvals {
        let request_id = approval.request_id.clone();
        let request_id2 = approval.request_id.clone();

        let approval_widget = container(
            column![
                text("Approval Required").size(13).color(theme::PEACH),
                text(&approval.description).size(14).color(theme::TEXT),
                row![
                    button(text("Approve").size(13).color(theme::GREEN))
                        .on_press(Message::Chat(ChatMessage::ApproveRequest {
                            request_id,
                        }))
                        .padding([6, 16]),
                    button(text("Deny").size(13).color(theme::RED))
                        .on_press(Message::Chat(ChatMessage::DenyRequest {
                            request_id: request_id2,
                        }))
                        .padding([6, 16]),
                ]
                .spacing(8),
            ]
            .spacing(6),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_SURFACE)),
            border: iced::Border {
                color: theme::PEACH,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        });

        elements.push(approval_widget.into());
    }

    let messages_col = Column::from_vec(elements).spacing(8).padding(12);

    scrollable(messages_col)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
