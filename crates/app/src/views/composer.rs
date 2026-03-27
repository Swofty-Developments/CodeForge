use iced::widget::{button, container, row, text, text_input, Space};
use iced::{Border, Element, Length};

use crate::message::{ComposerMessage, Message};
use crate::state::{AppState, SessionState};
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    if state.active_tab.is_none() {
        return Space::new().height(0).into();
    }

    let thread_id = state.active_tab.unwrap();
    let is_generating = state.is_thread_generating(thread_id);

    // Status pill
    let status_pill: Element<'_, Message> = {
        let (label, color) = match state.thread_session_state(thread_id) {
            Some(SessionState::Ready) => ("Ready", theme::GREEN),
            Some(SessionState::Generating) => ("Working", theme::SKY),
            Some(SessionState::Starting) => ("Connecting", theme::AMBER),
            Some(SessionState::Error) => ("Error", theme::RED),
            None => ("", theme::TEXT_TERTIARY),
        };
        if label.is_empty() {
            Space::new().width(0).into()
        } else {
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
    };

    let input = text_input("Message...", &state.composer_text)
        .on_input(|s| Message::Composer(ComposerMessage::TextChanged(s)))
        .on_submit(Message::Composer(ComposerMessage::Send))
        .padding([10, 14])
        .size(14)
        .width(Length::Fill);

    // Send button — round like t3code
    let send_btn: Element<'_, Message> = if is_generating {
        // Stop / generating indicator
        button(
            text("\u{25A0}").size(14).color(iced::Color::WHITE),
        )
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
        button(
            text("\u{2191}").size(16).color(iced::Color::WHITE),
        )
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
    };

    // Provider label
    let provider = text(format!("{}", state.selected_provider))
        .size(10)
        .color(theme::TEXT_TERTIARY);

    // Inner row: input + send
    let input_row = row![input, send_btn]
        .spacing(8)
        .align_y(iced::Alignment::Center);

    // Bottom bar: provider + status
    let bottom_bar = row![provider, Space::new().width(Length::Fill), status_pill]
        .align_y(iced::Alignment::Center)
        .padding([0, 4]);

    // Composer card
    let composer_card = container(
        column![input_row, bottom_bar].spacing(6),
    )
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

    // Center the composer
    container(composer_card)
        .width(Length::Fill)
        .padding([8, 20])
        .center_x(Length::Fill)
        .into()
}

use iced::widget::column;
