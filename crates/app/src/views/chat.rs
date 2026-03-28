use iced::widget::{button, center, column, container, row, scrollable, text, Column, Space};
use iced::{Border, Element, Length};

use crate::message::{ChatMessage, Message};
use crate::state::{AppState, MessageRole};
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let thread = match state.active_thread() {
        Some(t) => t,
        None => {
            // Empty state — centered welcome
            return center(
                column![
                    text("CodeForge").size(28).color(theme::TEXT_TERTIARY),
                    Space::new().height(8),
                    text("Select a thread or create a new one")
                        .size(14)
                        .color(theme::TEXT_TERTIARY),
                ]
                .align_x(iced::Alignment::Center),
            )
            .into();
        }
    };

    let thread_id = thread.id;
    let is_generating = state.is_thread_generating(thread_id);

    if thread.messages.is_empty() && !is_generating {
        return center(
            column![
                text("New conversation").size(18).color(theme::TEXT_SECONDARY),
                Space::new().height(4),
                text(format!("Using {}", state.selected_provider))
                    .size(12)
                    .color(theme::TEXT_TERTIARY),
            ]
            .spacing(0)
            .align_x(iced::Alignment::Center),
        )
        .into();
    }

    let mut elements: Vec<Element<'_, Message>> = thread
        .messages
        .iter()
        .map(|msg| match msg.role {
            MessageRole::User => {
                // Right-aligned bubble like t3code
                let bubble = container(
                    text(&msg.content).size(14).color(theme::TEXT),
                )
                .padding([10, 16])
                .max_width(560)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(theme::BG_USER_BUBBLE)),
                    border: Border {
                        color: theme::BORDER,
                        width: 1.0,
                        radius: theme::RADIUS_LG.into(),
                    },
                    ..Default::default()
                });

                row![Space::new().width(Length::Fill), bubble]
                    .width(Length::Fill)
                    .into()
            }
            MessageRole::Assistant => {
                // Left-aligned, flat — like t3code assistant messages
                let content_col = column![
                    text(&msg.content).size(14).color(theme::TEXT),
                ]
                .spacing(2);

                container(content_col)
                    .padding([8, 4])
                    .width(Length::Fill)
                    .into()
            }
            MessageRole::System => {
                // Centered system message
                let pill = container(
                    text(&msg.content).size(11).color(theme::TEXT_SECONDARY),
                )
                .padding([4, 12])
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(theme::BG_MUTED)),
                    border: Border {
                        color: theme::BORDER,
                        width: 1.0,
                        radius: theme::RADIUS_PILL.into(),
                    },
                    ..Default::default()
                });

                row![Space::new().width(Length::Fill), pill, Space::new().width(Length::Fill)]
                    .into()
            }
        })
        .collect();

    // Streaming indicator
    if is_generating {
        let indicator = container(
            row![
                text("\u{25CF}").size(8).color(theme::SKY),
                text("Generating...").size(12).color(theme::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .padding([6, 8]);
        elements.push(indicator.into());
    }

    // Pending approval prompts
    let approvals = state.active_thread_approvals();
    for approval in approvals {
        let request_id = approval.request_id.clone();
        let request_id2 = approval.request_id.clone();

        let approve_btn = button(
            text("Approve").size(12).color(theme::GREEN),
        )
        .on_press(Message::Chat(ChatMessage::ApproveRequest {
            request_id,
        }))
        .padding([5, 14])
        .style(|_theme, _status| button::Style {
            background: Some(iced::Background::Color(theme::BG_ACCENT)),
            text_color: theme::GREEN,
            border: Border {
                color: theme::GREEN,
                width: 1.0,
                radius: theme::RADIUS_SM.into(),
            },
            ..Default::default()
        });

        let deny_btn = button(
            text("Deny").size(12).color(theme::RED),
        )
        .on_press(Message::Chat(ChatMessage::DenyRequest {
            request_id: request_id2,
        }))
        .padding([5, 14])
        .style(|_theme, _status| button::Style {
            background: None,
            text_color: theme::RED,
            border: Border {
                color: theme::BORDER_STRONG,
                width: 1.0,
                radius: theme::RADIUS_SM.into(),
            },
            ..Default::default()
        });

        let approval_card = container(
            column![
                row![
                    text("\u{26A0}").size(12).color(theme::AMBER),
                    text("Approval Required").size(12).color(theme::AMBER),
                ]
                .spacing(6),
                text(&approval.description).size(13).color(theme::TEXT),
                row![approve_btn, deny_btn].spacing(8),
            ]
            .spacing(8),
        )
        .padding([12, 16])
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_CARD)),
            border: Border {
                color: theme::AMBER,
                width: 1.0,
                radius: theme::RADIUS_MD.into(),
            },
            ..Default::default()
        });

        elements.push(approval_card.into());
    }

    let messages_col = Column::from_vec(elements)
        .spacing(12)
        .padding([16, 20])
        .width(Length::Fill)
        .max_width(768);

    // Center the message column within the chat area
    let centered = container(messages_col)
        .width(Length::Fill)
        .center_x(Length::Fill);

    scrollable(centered)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
