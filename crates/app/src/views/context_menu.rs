use iced::widget::{button, column, container, text, Column};
use iced::{Border, Element, Length};

use crate::message::{Message, SidebarMessage};
use crate::theme;

pub fn thread_menu(thread_id: crate::state::ThreadId) -> Element<'static, Message> {
    let menu = column![
        menu_btn(
            "Rename",
            Message::Sidebar(SidebarMessage::StartRename(thread_id))
        ),
        menu_btn(
            "Delete",
            Message::Sidebar(SidebarMessage::DeleteThread(thread_id))
        ),
        color_swatches(move |c| Message::Sidebar(SidebarMessage::SetThreadColor(thread_id, c))),
    ];
    wrap(menu)
}

pub fn project_menu(project_id: crate::state::ProjectId) -> Element<'static, Message> {
    let menu = column![
        menu_btn(
            "Rename Group",
            Message::Sidebar(SidebarMessage::RenameProject(project_id))
        ),
        menu_btn(
            "New Thread",
            Message::Sidebar(SidebarMessage::NewThreadInProject(project_id))
        ),
        menu_btn(
            "Delete Group",
            Message::Sidebar(SidebarMessage::DeleteProject(project_id))
        ),
        color_swatches(
            move |c| Message::Sidebar(SidebarMessage::SetProjectColor(project_id, c))
        ),
    ];
    wrap(menu)
}

fn menu_btn(label: &str, msg: Message) -> Element<'static, Message> {
    let label = label.to_string();
    button(text(label).size(12).color(theme::TEXT))
        .on_press(msg)
        .padding([5, 12])
        .width(Length::Fill)
        .style(|_theme, status| button::Style {
            background: match status {
                button::Status::Hovered => Some(iced::Background::Color(theme::BG_ACCENT)),
                _ => None,
            },
            text_color: theme::TEXT,
            border: Border::default(),
            ..Default::default()
        })
        .into()
}

fn color_swatches<F: Fn(Option<String>) -> Message + 'static>(
    on_select: F,
) -> Element<'static, Message> {
    let swatches: Vec<Element<'static, Message>> = theme::THREAD_COLORS
        .iter()
        .map(|&(hex, color)| {
            let hex = hex.to_string();
            button(text("\u{25CF}").size(14).color(color))
                .on_press(on_select(Some(hex)))
                .padding([2, 3])
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
                })
                .into()
        })
        .collect();

    let clear = button(text("\u{00D7}").size(14).color(theme::TEXT_TERTIARY))
        .on_press(on_select(None))
        .padding([2, 3])
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

    let mut swatch_row = iced::widget::Row::from_vec(swatches).spacing(2);
    swatch_row = swatch_row.push(clear);

    container(
        column![
            text("Color").size(11).color(theme::TEXT_TERTIARY),
            swatch_row,
        ]
        .spacing(4),
    )
    .padding([5, 12])
    .into()
}

fn wrap(menu: Column<'static, Message>) -> Element<'static, Message> {
    container(menu.spacing(1))
        .padding(4)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_CARD)),
            border: Border {
                color: theme::BORDER_STRONG,
                width: 1.0,
                radius: theme::RADIUS_MD.into(),
            },
            ..Default::default()
        })
        .into()
}
