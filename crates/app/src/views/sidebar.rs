use iced::widget::{button, column, container, scrollable, text, Column};
use iced::{Element, Length};

use crate::message::{Message, SidebarMessage};
use crate::state::AppState;
use crate::theme;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = container(
        text("CodeForge")
            .size(20)
            .color(theme::PRIMARY),
    )
    .padding(16);

    let project_list: Element<'_, Message> = if state.projects.is_empty() {
        column![text("No projects yet").size(13).color(theme::SUBTEXT)]
            .padding([16, 16])
            .into()
    } else {
        let items: Vec<Element<'_, Message>> = state
            .projects
            .iter()
            .flat_map(|project| {
                let mut elements: Vec<Element<'_, Message>> = vec![
                    text(&project.name)
                        .size(11)
                        .color(theme::SUBTEXT)
                        .into(),
                ];
                for thread in &project.threads {
                    let is_active = state.active_tab == Some(thread.id);
                    let thread_btn = button(
                        text(&thread.title)
                            .size(14)
                            .color(if is_active { theme::PRIMARY } else { theme::TEXT }),
                    )
                    .on_press(Message::Sidebar(SidebarMessage::SelectThread(thread.id)))
                    .padding([6, 12])
                    .width(Length::Fill);
                    elements.push(thread_btn.into());
                }
                elements
            })
            .collect();
        Column::from_vec(items).spacing(2).padding([8, 8]).into()
    };

    let threads_scrollable = scrollable(project_list).height(Length::Fill);

    let new_thread_btn = button(text("+ New Thread").size(14).color(theme::TEXT))
        .on_press(Message::Sidebar(SidebarMessage::NewThread))
        .padding([8, 16])
        .width(Length::Fill);

    let settings_btn = button(text("Settings").size(13).color(theme::SUBTEXT))
        .on_press(Message::Settings(crate::message::SettingsMessage::Open))
        .padding([6, 16])
        .width(Length::Fill);

    let content = column![header, threads_scrollable, new_thread_btn, settings_btn]
        .spacing(4)
        .height(Length::Fill);

    container(content)
        .width(250)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BG_SURFACE)),
            border: iced::Border {
                color: theme::BORDER,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}
