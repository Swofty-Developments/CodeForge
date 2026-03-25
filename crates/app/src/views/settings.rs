use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

use crate::message::{Message, SettingsMessage};
use crate::state::{AppState, ApprovalMode};
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

    // Approval mode toggle
    let supervised_btn = button(
        text("Supervised")
            .size(13)
            .color(if state.approval_mode == ApprovalMode::Supervised {
                theme::PRIMARY
            } else {
                theme::SUBTEXT
            }),
    )
    .on_press(Message::Settings(SettingsMessage::ApprovalModeChanged(
        ApprovalMode::Supervised,
    )))
    .padding([6, 12]);

    let auto_btn = button(
        text("Auto-approve")
            .size(13)
            .color(if state.approval_mode == ApprovalMode::AutoApprove {
                theme::PRIMARY
            } else {
                theme::SUBTEXT
            }),
    )
    .on_press(Message::Settings(SettingsMessage::ApprovalModeChanged(
        ApprovalMode::AutoApprove,
    )))
    .padding([6, 12]);

    let approval_section = column![
        text("Approval Mode").size(14).color(theme::TEXT),
        row![supervised_btn, auto_btn].spacing(8),
    ]
    .spacing(4);

    // Provider path configuration
    let claude_path_section = column![
        text("Claude Binary Path").size(14).color(theme::TEXT),
        text_input("claude", &state.claude_path)
            .on_input(|s| Message::Settings(SettingsMessage::ClaudePathChanged(s)))
            .padding(8)
            .size(13)
            .width(Length::Fill),
    ]
    .spacing(4);

    let codex_path_section = column![
        text("Codex Binary Path").size(14).color(theme::TEXT),
        text_input("codex", &state.codex_path)
            .on_input(|s| Message::Settings(SettingsMessage::CodexPathChanged(s)))
            .padding(8)
            .size(13)
            .width(Length::Fill),
    ]
    .spacing(4);

    let content = column![
        header,
        close_btn,
        provider_section,
        approval_section,
        claude_path_section,
        codex_path_section,
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
