use crate::message::{Message, TabMessage};
use crate::App;

pub fn handle(app: &mut App, msg: TabMessage) -> iced::Task<Message> {
    match msg {
        TabMessage::Select(id) => {
            app.state.active_tab = Some(id);
        }
        TabMessage::Close(id) => {
            app.state.open_tabs.retain(|&t| t != id);
            if app.state.active_tab == Some(id) {
                app.state.active_tab = app.state.open_tabs.last().copied();
            }
        }
        TabMessage::StartDrag(id) => {
            app.state.dragging_tab = Some(id);
        }
        TabMessage::DragOver(target_idx) => {
            if let Some(dragged_id) = app.state.dragging_tab {
                if let Some(src_idx) = app.state.open_tabs.iter().position(|&t| t == dragged_id) {
                    if src_idx != target_idx && target_idx < app.state.open_tabs.len() {
                        let id = app.state.open_tabs.remove(src_idx);
                        app.state.open_tabs.insert(target_idx, id);
                    }
                }
            }
        }
        TabMessage::EndDrag => {
            app.state.dragging_tab = None;
        }
    }
    iced::Task::none()
}
