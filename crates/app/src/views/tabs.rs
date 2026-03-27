use iced::widget::{button, container, row, text, Row, Space};
use iced::{Border, Element, Length};

use crate::message::{Message, TabMessage};
use crate::state::AppState;
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    if state.open_tabs.is_empty() {
        return Space::new().height(0).into();
    }

    let tabs: Vec<Element<'_, Message>> = state
        .open_tabs
        .iter()
        .filter_map(|&tab_id| {
            let thread = state.find_thread(tab_id)?;
            let is_active = state.active_tab == Some(tab_id);

            let label = text(&thread.title)
                .size(12)
                .color(if is_active {
                    theme::TEXT
                } else {
                    theme::TEXT_SECONDARY
                });

            let close = button(text("\u{00D7}").size(14).color(theme::TEXT_TERTIARY))
                .on_press(Message::Tab(TabMessage::Close(tab_id)))
                .padding([0, 4])
                .style(|_theme, _status| button::Style {
                    background: None,
                    text_color: theme::TEXT_TERTIARY,
                    ..Default::default()
                });

            let tab_content = row![label, close]
                .spacing(6)
                .align_y(iced::Alignment::Center);

            let tab = button(tab_content)
                .on_press(Message::Tab(TabMessage::Select(tab_id)))
                .padding([6, 12])
                .style(move |_theme, _status| button::Style {
                    background: if is_active {
                        Some(iced::Background::Color(theme::BG_BASE))
                    } else {
                        None
                    },
                    text_color: theme::TEXT,
                    border: Border {
                        color: if is_active {
                            theme::BORDER_STRONG
                        } else {
                            iced::Color::TRANSPARENT
                        },
                        width: if is_active { 1.0 } else { 0.0 },
                        radius: theme::RADIUS_SM.into(),
                    },
                    ..Default::default()
                });

            Some(tab.into())
        })
        .collect();

    let tab_bar = Row::from_vec(tabs).spacing(1).padding([0, 8]);

    container(tab_bar)
        .width(Length::Fill)
        .padding([6, 0])
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_MUTED)),
            border: Border {
                color: theme::BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}
