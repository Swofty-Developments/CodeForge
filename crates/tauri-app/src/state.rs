use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use codeforge_persistence::Database;
use codeforge_session::SessionManager;
use uuid::Uuid;

/// Session ID from the session manager (raw UUID).
pub type MgrSessionId = Uuid;

pub struct TauriState {
    pub db: Arc<Mutex<Database>>,
    pub session_manager: tokio::sync::Mutex<SessionManager>,
    /// Maps persistence ThreadId -> session manager SessionId.
    pub thread_sessions: tokio::sync::Mutex<HashMap<codeforge_persistence::ThreadId, MgrSessionId>>,
}
