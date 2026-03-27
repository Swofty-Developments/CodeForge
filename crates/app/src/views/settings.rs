use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Border, Element, Length};

use crate::message::{Message, SettingsMessage};
use crate::state::{AppState, ApprovalMode};
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = row![
        text("Settings").size(18).color(theme::TEXT),
        Space::new().width(Length::Fill),
        button(text("\u{00D7}").size(18).color(theme::TEXT_SECONDARY))
            .on_press(Message::Settings(SettingsMessage::Close))
            .padding([2, 8])
            .style(|_theme, _status| button::Style {
                background: None,
                text_color: theme::TEXT_SECONDARY,
                border: Border {
                    radius: theme::RADIUS_SM.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    ]
    .align_y(iced::Alignment::Center);

    // Provider section
    let provider_section = column![
        text("Default Provider")
            .size(11)
            .color(theme::TEXT_TERTIARY),
        text(format!("{}", state.selected_provider))
            .size(14)
            .color(theme::TEXT),
    ]
    .spacing(4);

    // Approval mode toggle
    let is_supervised = state.approval_mode == ApprovalMode::Supervised;
    let supervised_btn = button(
        text("Supervised")
            .size(12)
            .color(if is_supervised { theme::TEXT } else { theme::TEXT_SECONDARY }),
    )
    .on_press(Message::Settings(SettingsMessage::ApprovalModeChanged(ApprovalMode::Supervised)))
    .padding([6, 14])
    .style(move |_theme, _status| button::Style {
        background: if is_supervised {
            Some(iced::Background::Color(theme::BG_ACCENT))
        } else {
            None
        },
        text_color: if is_supervised { theme::TEXT } else { theme::TEXT_SECONDARY },
        border: Border {
            color: if is_supervised { theme::BORDER_STRONG } else { theme::BORDER },
            width: 1.0,
            radius: theme::RADIUS_SM.into(),
        },
        ..Default::default()
    });

    let is_auto = state.approval_mode == ApprovalMode::AutoApprove;
    let auto_btn = button(
        text("Auto-approve")
            .size(12)
            .color(if is_auto { theme::TEXT } else { theme::TEXT_SECONDARY }),
    )
    .on_press(Message::Settings(SettingsMessage::ApprovalModeChanged(ApprovalMode::AutoApprove)))
    .padding([6, 14])
    .style(move |_theme, _status| button::Style {
        background: if is_auto {
            Some(iced::Background::Color(theme::BG_ACCENT))
        } else {
            None
        },
        text_color: if is_auto { theme::TEXT } else { theme::TEXT_SECONDARY },
        border: Border {
            color: if is_auto { theme::BORDER_STRONG } else { theme::BORDER },
            width: 1.0,
            radius: theme::RADIUS_SM.into(),
        },
        ..Default::default()
    });

    let approval_section = column![
        text("Approval Mode")
            .size(11)
            .color(theme::TEXT_TERTIARY),
        row![supervised_btn, auto_btn].spacing(6),
    ]
    .spacing(6);

    // Path inputs
    let claude_section = column![
        text("Claude Binary")
            .size(11)
            .color(theme::TEXT_TERTIARY),
        text_input("claude", &state.claude_path)
            .on_input(|s| Message::Settings(SettingsMessage::ClaudePathChanged(s)))
            .padding([8, 12])
            .size(13)
            .width(Length::Fill),
    ]
    .spacing(4);

    let codex_section = column![
        text("Codex Binary")
            .size(11)
            .color(theme::TEXT_TERTIARY),
        text_input("codex", &state.codex_path)
            .on_input(|s| Message::Settings(SettingsMessage::CodexPathChanged(s)))
            .padding([8, 12])
            .size(13)
            .width(Length::Fill),
    ]
    .spacing(4);

    // Separator
    let sep = container(Space::new().height(1))
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BORDER)),
            ..Default::default()
        });

    let panel = container(
        column![
            header,
            sep,
            provider_section,
            approval_section,
            claude_section,
            codex_section,
        ]
        .spacing(20)
        .padding(24)
        .width(420),
    )
    .style(|_theme| container::Style {
        background: Some(iced::Background::Color(theme::BG_CARD)),
        border: Border {
            color: theme::BORDER_STRONG,
            width: 1.0,
            radius: theme::RADIUS_LG.into(),
        },
        ..Default::default()
    });

    // Overlay backdrop
    container(
        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_theme| container::Style {
        background: Some(iced::Background::Color(iced::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.65,
        })),
        ..Default::default()
    })
    .into()
}
