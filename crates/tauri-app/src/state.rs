use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use codeforge_persistence::Database;
use codeforge_session::SessionManager;
use uuid::Uuid;

pub type SessionId = Uuid;
pub type ThreadId = Uuid;

pub struct TauriState {
    pub db: Arc<Mutex<Database>>,
    pub session_manager: tokio::sync::Mutex<SessionManager>,
    pub thread_sessions: tokio::sync::Mutex<HashMap<ThreadId, SessionId>>,
}
