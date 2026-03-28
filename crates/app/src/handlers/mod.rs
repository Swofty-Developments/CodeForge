mod agent;
mod chat;
mod composer;
mod db;
mod popup;
mod session;
mod settings;
mod sidebar;
mod tabs;

use crate::message::Message;
use crate::App;

pub fn update(app: &mut App, message: Message) -> iced::Task<Message> {
    match message {
        Message::DbLoaded(result) => db::handle(app, result),
        Message::Sidebar(msg) => sidebar::handle(app, msg),
        Message::Chat(msg) => chat::handle(app, msg),
        Message::Composer(msg) => composer::handle(app, msg),
        Message::Tab(msg) => tabs::handle(app, msg),
        Message::Settings(msg) => settings::handle(app, msg),
        Message::Agent(msg) => agent::handle(app, msg),
        Message::Popup(msg) => popup::handle(app, msg),
        Message::SessionCreated {
            thread_id,
            session_id,
            result,
        } => session::handle_created(app, thread_id, session_id, result),
        Message::MessageSent { thread_id, result } => {
            session::handle_message_sent(app, thread_id, result)
        }
    }
}
