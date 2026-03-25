use iced::widget::{button, container, row, text, Row};
use iced::{Element, Length};

use crate::message::{Message, TabMessage};
use crate::state::AppState;
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    if state.open_tabs.is_empty() {
        return container(text("")).height(0).into();
    }

    let tabs: Vec<Element<'_, Message>> = state
        .open_tabs
        .iter()
        .filter_map(|&tab_id| {
            let thread = state.find_thread(tab_id)?;
            let is_active = state.active_tab == Some(tab_id);

            let label = text(&thread.title)
                .size(13)
                .color(if is_active { theme::TEXT } else { theme::SUBTEXT });

            let close = button(text("x").size(11).color(theme::SUBTEXT))
                .on_press(Message::Tab(TabMessage::Close(tab_id)))
                .padding([2, 6]);

            let tab_content = row![label, close].spacing(8).align_y(iced::Alignment::Center);

            let tab = button(tab_content)
                .on_press(Message::Tab(TabMessage::Select(tab_id)))
                .padding([6, 12])
                .style(move |_theme, _status| {
                    button::Style {
                        background: Some(iced::Background::Color(if is_active {
                            theme::BG_BASE
                        } else {
                            theme::BG_SURFACE
                        })),
                        text_color: theme::TEXT,
                        border: iced::Border {
                            color: if is_active { theme::PRIMARY } else { theme::BORDER },
                            width: if is_active { 0.0 } else { 0.0 },
                            radius: 0.0.into(),
                        },
                        ..Default::default()
                    }
                });

            Some(tab.into())
        })
        .collect();

    let tab_bar = Row::from_vec(tabs).spacing(1);

    container(tab_bar)
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
