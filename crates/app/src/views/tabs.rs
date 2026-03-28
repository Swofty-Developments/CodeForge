use iced::widget::{button, container, mouse_area, row, text, Row, Space};
use iced::{Border, Element, Length};

use crate::message::{Message, TabMessage};
use crate::state::AppState;
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    if state.open_tabs.is_empty() {
        return Space::new().height(0).into();
    }

    let is_dragging = state.dragging_tab.is_some();

    let tabs: Vec<Element<'_, Message>> = state
        .open_tabs
        .iter()
        .enumerate()
        .filter_map(|(idx, &tab_id)| {
            let thread = state.find_thread(tab_id)?;
            let is_active = state.active_tab == Some(tab_id);
            let is_being_dragged = state.dragging_tab == Some(tab_id);

            let label = text(&thread.title)
                .size(12)
                .color(if is_being_dragged {
                    theme::PRIMARY
                } else if is_active {
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

            let thread_color = thread.color.as_deref().and_then(theme::hex_to_color);

            // When dragging, the whole tab is a drop target
            if is_dragging && !is_being_dragged {
                let drop_target = button(
                    row![label, close]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                )
                .on_press(Message::Tab(TabMessage::DragOver(idx)))
                .padding([6, 12])
                .style(move |_theme, status| {
                    let border_color = match status {
                        button::Status::Hovered => theme::PRIMARY,
                        _ if is_active => theme::BORDER_STRONG,
                        _ => iced::Color::TRANSPARENT,
                    };
                    button::Style {
                        background: if is_active {
                            Some(iced::Background::Color(theme::BG_BASE))
                        } else {
                            None
                        },
                        text_color: theme::TEXT,
                        border: Border {
                            color: border_color,
                            width: 1.0,
                            radius: theme::RADIUS_SM.into(),
                        },
                        ..Default::default()
                    }
                });
                return Some(drop_target.into());
            }

            // Drag handle + select button + close
            let drag_handle = mouse_area(
                text("\u{2261}")
                    .size(11)
                    .color(theme::TEXT_TERTIARY),
            )
            .on_press(Message::Tab(TabMessage::StartDrag(tab_id)))
            .interaction(iced::mouse::Interaction::Grab);

            let select_btn = button(label)
                .on_press(Message::Tab(TabMessage::Select(tab_id)))
                .padding([6, 8])
                .style(move |_theme, _status| button::Style {
                    background: None,
                    text_color: theme::TEXT,
                    border: Border::default(),
                    ..Default::default()
                });

            let tab_row = row![drag_handle, select_btn, close]
                .spacing(2)
                .align_y(iced::Alignment::Center);

            let tab = container(tab_row)
                .padding([0, 4])
                .style(move |_theme| container::Style {
                    background: if is_active {
                        Some(iced::Background::Color(theme::BG_BASE))
                    } else {
                        None
                    },
                    border: Border {
                        color: if let Some(c) = thread_color {
                            c
                        } else if is_active {
                            theme::BORDER_STRONG
                        } else {
                            iced::Color::TRANSPARENT
                        },
                        width: if is_active || thread_color.is_some() {
                            1.0
                        } else {
                            0.0
                        },
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
