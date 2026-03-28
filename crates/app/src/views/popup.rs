use iced::widget::{button, column, container, row, text, Space};
use iced::{Border, Element, Length};

use crate::message::{ComposerMessage, Message, PopupMessage};
use crate::state::{AppState, PendingPopup};
use crate::theme;

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    let popup = state.pending_popup.as_ref()?;

    let content: Element<'_, Message> = match popup {
        PendingPopup::ConfirmDeleteGroup { project_id } => {
            let name = state
                .projects
                .iter()
                .find(|p| p.id == *project_id)
                .map(|p| p.name.as_str())
                .unwrap_or("this group");

            column![
                text(format!("Delete \"{}\"?", name))
                    .size(16)
                    .color(theme::TEXT),
                Space::new().height(8),
                text("What should happen to the threads in this group?")
                    .size(13)
                    .color(theme::TEXT_SECONDARY),
                Space::new().height(16),
                row![
                    popup_btn(
                        "Delete All",
                        theme::RED,
                        Message::Popup(PopupMessage::ConfirmDeleteGroup {
                            delete_threads: true
                        })
                    ),
                    popup_btn(
                        "Keep Threads",
                        theme::PRIMARY,
                        Message::Popup(PopupMessage::ConfirmDeleteGroup {
                            delete_threads: false
                        })
                    ),
                    popup_btn(
                        "Cancel",
                        theme::TEXT_TERTIARY,
                        Message::Popup(PopupMessage::CancelPopup)
                    ),
                ]
                .spacing(8),
            ]
            .into()
        }
        PendingPopup::ConfirmNewGroup { directory } => {
            let dir_name = std::path::Path::new(directory)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| directory.clone());
            let dir_clone = directory.clone();

            column![
                text(format!("Create new group for \"{}\"?", dir_name))
                    .size(16)
                    .color(theme::TEXT),
                Space::new().height(4),
                text(directory)
                    .size(11)
                    .color(theme::TEXT_TERTIARY),
                Space::new().height(16),
                row![
                    popup_btn(
                        "Create Group",
                        theme::PRIMARY,
                        Message::Composer(ComposerMessage::ConfirmNewGroup(dir_clone))
                    ),
                    popup_btn(
                        "Cancel",
                        theme::TEXT_TERTIARY,
                        Message::Popup(PopupMessage::CancelPopup)
                    ),
                ]
                .spacing(8),
            ]
            .into()
        }
    };

    let panel = container(content)
        .padding(24)
        .max_width(400)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_CARD)),
            border: Border {
                color: theme::BORDER_STRONG,
                width: 1.0,
                radius: theme::RADIUS_LG.into(),
            },
            ..Default::default()
        });

    let overlay = container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.65,
            })),
            ..Default::default()
        });

    Some(overlay.into())
}

fn popup_btn(label: &str, color: iced::Color, msg: Message) -> Element<'static, Message> {
    let label = label.to_string();
    button(text(label).size(13).color(color))
        .on_press(msg)
        .padding([8, 16])
        .style(move |_theme, status| button::Style {
            background: match status {
                button::Status::Hovered => Some(iced::Background::Color(theme::BG_ACCENT)),
                _ => Some(iced::Background::Color(theme::BG_MUTED)),
            },
            border: Border {
                color: theme::BORDER,
                width: 1.0,
                radius: theme::RADIUS_MD.into(),
            },
            ..Default::default()
        })
        .into()
}
