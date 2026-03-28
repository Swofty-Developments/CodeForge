use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, Column, Space,
};
use iced::{Border, Element, Length};

use crate::message::{Message, SettingsMessage, SidebarMessage};
use crate::state::{AppState, ContextMenu};
use crate::theme;
use crate::views::context_menu;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = header_view();
    let project_list = project_list_view(state);
    let threads_scrollable = scrollable(project_list).height(Length::Fill);
    let bottom = bottom_view();

    let content = column![header, threads_scrollable, bottom].height(Length::Fill);

    container(content)
        .width(state.sidebar_width)
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

fn header_view() -> Element<'static, Message> {
    container(
        row![
            text("CodeForge").size(14).color(theme::TEXT),
            Space::new().width(Length::Fill),
            button(text("\u{2699}").size(15).color(theme::TEXT_TERTIARY))
                .on_press(Message::Settings(SettingsMessage::Open))
                .padding([2, 4])
                .style(|_theme, status| button::Style {
                    background: match status {
                        button::Status::Hovered => {
                            Some(iced::Background::Color(theme::BG_ACCENT))
                        }
                        _ => None,
                    },
                    border: Border {
                        radius: theme::RADIUS_SM.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([14, 16])
    .into()
}

fn project_list_view(state: &AppState) -> Element<'_, Message> {
    if state.projects.is_empty() {
        return container(
            column![
                text("No threads yet").size(14).color(theme::TEXT),
                text("Click + below to start")
                    .size(12)
                    .color(theme::TEXT_SECONDARY),
            ]
            .spacing(4)
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding([32, 16])
        .center_x(Length::Fill)
        .into();
    }

    let is_dragging = state.dragging_thread.is_some();
    let mut items: Vec<Element<'_, Message>> = Vec::new();

    for project in &state.projects {
        let is_uncategorized = project.path == ".";

        items.push(project_header(state, project, is_dragging && !is_uncategorized));

        if matches!(&state.context_menu, Some(ContextMenu::Project(id)) if *id == project.id) {
            items.push(context_menu::project_menu(project.id));
        }

        if !project.collapsed || is_uncategorized {
            for thread in &project.threads {
                items.push(thread_item(state, thread, is_uncategorized));

                if matches!(&state.context_menu, Some(ContextMenu::Thread(id)) if *id == thread.id)
                {
                    items.push(context_menu::thread_menu(thread.id));
                }
            }
        }
    }

    Column::from_vec(items).spacing(1).width(Length::Fill).into()
}

fn project_header<'a>(
    state: &'a AppState,
    project: &'a crate::state::Project,
    is_drop_target: bool,
) -> Element<'a, Message> {
    let is_renaming = state
        .renaming_project
        .as_ref()
        .map(|(id, _)| *id == project.id)
        .unwrap_or(false);

    if is_renaming {
        let rename_text = state
            .renaming_project
            .as_ref()
            .map(|(_, t)| t.as_str())
            .unwrap_or("");
        return container(
            text_input("Group name...", rename_text)
                .on_input(|s| Message::Sidebar(SidebarMessage::ProjectRenameTextChanged(s)))
                .on_submit(Message::Sidebar(SidebarMessage::ConfirmProjectRename))
                .size(11)
                .padding([3, 8]),
        )
        .padding([4, 10])
        .into();
    }

    let color = project
        .color
        .as_deref()
        .and_then(theme::hex_to_color)
        .unwrap_or(theme::TEXT_TERTIARY);

    let collapse_icon = if project.collapsed { "\u{25B6}" } else { "\u{25BC}" };

    let project_id = project.id;

    let collapse_part = button(
        row![
            text(collapse_icon).size(8).color(theme::TEXT_TERTIARY),
            text(project.name.to_uppercase())
                .size(10)
                .color(color),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .on_press(if is_drop_target {
        Message::Sidebar(SidebarMessage::DropOnProject(project_id))
    } else {
        Message::Sidebar(SidebarMessage::ToggleProjectCollapse(project_id))
    })
    .padding([6, 8])
    .width(Length::Fill)
    .style(move |_theme, status| {
        let bg = if is_drop_target {
            match status {
                button::Status::Hovered => Some(iced::Background::Color(theme::PRIMARY)),
                _ => Some(iced::Background::Color(theme::BG_ACCENT)),
            }
        } else {
            match status {
                button::Status::Hovered => Some(iced::Background::Color(theme::BG_MUTED)),
                _ => None,
            }
        };
        button::Style {
            background: bg,
            border: if is_drop_target {
                Border {
                    color: theme::PRIMARY,
                    width: 1.0,
                    radius: theme::RADIUS_SM.into(),
                }
            } else {
                Border::default()
            },
            ..Default::default()
        }
    });

    let add_btn = button(text("+").size(12).color(theme::TEXT_TERTIARY))
        .on_press(Message::Sidebar(SidebarMessage::NewThreadInProject(
            project_id,
        )))
        .padding([6, 8])
        .style(|_theme, status| button::Style {
            background: match status {
                button::Status::Hovered => Some(iced::Background::Color(theme::BG_ACCENT)),
                _ => None,
            },
            border: Border {
                radius: theme::RADIUS_SM.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let header_row = row![collapse_part, add_btn]
        .width(Length::Fill)
        .align_y(iced::Alignment::Center);

    container(
        mouse_area(header_row)
            .on_right_press(Message::Sidebar(SidebarMessage::ShowProjectContextMenu(
                project_id,
            ))),
    )
    .width(Length::Fill)
    .into()
}

fn thread_item<'a>(state: &'a AppState, thread: &'a crate::state::Thread, is_uncategorized: bool) -> Element<'a, Message> {
    let thread_id = thread.id;
    let is_active = state.active_tab == Some(thread_id);
    let is_renaming = state
        .renaming_thread
        .as_ref()
        .map(|(id, _)| *id == thread_id)
        .unwrap_or(false);

    let thread_color = thread.color.as_deref().and_then(theme::hex_to_color);

    let dot: Element<'_, Message> = if let Some(color) = thread_color {
        text("\u{25CF}").size(8).color(color).into()
    } else if state.has_active_session(thread_id) {
        let session_state = state.thread_session_state(thread_id);
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

    if is_renaming {
        let rename_text = state
            .renaming_thread
            .as_ref()
            .map(|(_, t)| t.as_str())
            .unwrap_or("");
        return container(
            row![
                dot,
                text_input("Thread name...", rename_text)
                    .on_input(|s| Message::Sidebar(SidebarMessage::RenameTextChanged(s)))
                    .on_submit(Message::Sidebar(SidebarMessage::ConfirmRename))
                    .size(13)
                    .padding([4, 8]),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding([1, 6])
        .into();
    }

    let label = row![
        dot,
        text(&thread.title).size(13).color(if is_active {
            theme::TEXT
        } else {
            theme::TEXT_SECONDARY
        }),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let left_border_color = thread_color.unwrap_or(iced::Color::TRANSPARENT);

    let thread_btn = button(label)
        .on_press(Message::Sidebar(SidebarMessage::SelectThread(thread_id)))
        .padding([5, 14])
        .width(Length::Fill)
        .style(move |_theme, _status| button::Style {
            background: None,
            text_color: theme::TEXT,
            border: Border {
                color: left_border_color,
                width: if thread_color.is_some() { 2.0 } else { 0.0 },
                radius: theme::RADIUS_SM.into(),
            },
            ..Default::default()
        });

    let is_being_dragged = state.dragging_thread == Some(thread_id);

    let thread_row: Element<'_, Message> = if is_uncategorized {
        let drag_handle = mouse_area(
            container(
                text("\u{2261}").size(12).color(if is_being_dragged {
                    theme::PRIMARY
                } else {
                    theme::TEXT_TERTIARY
                }),
            )
            .padding([5, 6]),
        )
        .on_press(Message::Sidebar(SidebarMessage::StartDragThread(thread_id)))
        .interaction(iced::mouse::Interaction::Grab);

        row![thread_btn, drag_handle]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center)
            .into()
    } else {
        thread_btn.into()
    };

    container(
        mouse_area(thread_row)
            .on_right_press(Message::Sidebar(SidebarMessage::ShowThreadContextMenu(
                thread_id,
            )))
            .on_double_click(Message::Sidebar(SidebarMessage::StartRename(thread_id))),
    )
    .width(Length::Fill)
    .style(move |_theme| container::Style {
        background: if is_active {
            Some(iced::Background::Color(theme::BG_ACCENT))
        } else {
            None
        },
        border: Border {
            radius: theme::RADIUS_SM.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn bottom_view() -> Element<'static, Message> {
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

    container(new_thread_btn).padding([6, 4]).into()
}

