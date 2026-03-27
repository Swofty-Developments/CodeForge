use iced::widget::{button, column, container, row, scrollable, text, Column, Space};
use iced::{Border, Element, Length};

use crate::message::{Message, SettingsMessage, SidebarMessage};
use crate::state::AppState;
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    // ── Header ──────────────────────────────────────────────
    let header = container(
        row![
            text("CodeForge").size(15).color(theme::TEXT),
            Space::new().width(Length::Fill),
            text("v0.1").size(10).color(theme::TEXT_TERTIARY),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([14, 16]);

    // ── Project / thread list ───────────────────────────────
    let project_list: Element<'_, Message> = if state.projects.is_empty() {
        container(
            column![
                text("No threads yet").size(12).color(theme::TEXT_TERTIARY),
                text("Click + to start").size(11).color(theme::TEXT_TERTIARY),
            ]
            .spacing(4)
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding([32, 16])
        .center_x(Length::Fill)
        .into()
    } else {
        let items: Vec<Element<'_, Message>> = state
            .projects
            .iter()
            .flat_map(|project| {
                let mut elements: Vec<Element<'_, Message>> = vec![container(
                    text(project.name.to_uppercase())
                        .size(10)
                        .color(theme::TEXT_TERTIARY),
                )
                .padding([8, 14])
                .into()];

                for thread in &project.threads {
                    let is_active = state.active_tab == Some(thread.id);
                    let has_session = state.has_active_session(thread.id);

                    let dot: Element<'_, Message> = if has_session {
                        let session_state = state.thread_session_state(thread.id);
                        let dot_color = match session_state {
                            Some(crate::state::SessionState::Ready) => theme::GREEN,
                            Some(crate::state::SessionState::Generating)
                            | Some(crate::state::SessionState::Starting) => theme::SKY,
                            Some(crate::state::SessionState::Error) => theme::RED,
                            None => theme::TEXT_TERTIARY,
                        };
                        text("\u{25CF}").size(7).color(dot_color).into()
                    } else {
                        Space::new().width(7).into()
                    };

                    let label = row![
                        dot,
                        text(&thread.title)
                            .size(13)
                            .color(if is_active {
                                theme::TEXT
                            } else {
                                theme::TEXT_SECONDARY
                            }),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center);

                    let thread_btn = button(label)
                        .on_press(Message::Sidebar(SidebarMessage::SelectThread(thread.id)))
                        .padding([5, 14])
                        .width(Length::Fill)
                        .style(move |_theme, _status| button::Style {
                            background: if is_active {
                                Some(iced::Background::Color(theme::BG_ACCENT))
                            } else {
                                None
                            },
                            text_color: theme::TEXT,
                            border: Border {
                                radius: theme::RADIUS_SM.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        });

                    elements.push(thread_btn.into());
                }
                elements
            })
            .collect();
        Column::from_vec(items).spacing(1).into()
    };

    let threads_scrollable = scrollable(project_list).height(Length::Fill);

    // ── Bottom actions ──────────────────────────────────────
    let new_thread_btn = button(
        row![
            text("+").size(16).color(theme::PRIMARY),
            text("New Thread").size(13).color(theme::TEXT_SECONDARY),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::Sidebar(SidebarMessage::NewThread))
    .padding([8, 14])
    .width(Length::Fill)
    .style(|_theme, _status| button::Style {
        background: None,
        text_color: theme::TEXT,
        border: Border {
            radius: theme::RADIUS_SM.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let settings_btn = button(
        text("Settings").size(12).color(theme::TEXT_TERTIARY),
    )
    .on_press(Message::Settings(SettingsMessage::Open))
    .padding([6, 14])
    .width(Length::Fill)
    .style(|_theme, _status| button::Style {
        background: None,
        text_color: theme::TEXT_TERTIARY,
        border: Border::default(),
        ..Default::default()
    });

    let bottom = container(
        column![new_thread_btn, settings_btn].spacing(2),
    )
    .padding([6, 4]);

    // ── Assemble ────────────────────────────────────────────
    let content = column![header, threads_scrollable, bottom]
        .height(Length::Fill);

    container(content)
        .width(240)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_SURFACE)),
            border: Border {
                color: theme::BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}
