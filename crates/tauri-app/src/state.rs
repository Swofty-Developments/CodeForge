use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use forge_daemon::DaemonHandle;
use forge_index::FeatureIndex;
use forge_session::SessionManager;
use forge_timeline::TimelineStore;
use tokio::sync::RwLock;

use crate::db::Database;

/// Everything running for one open repository.
#[allow(dead_code)] // scaffold: consumed once command bodies are implemented
pub struct RepoRuntime {
    pub daemon: DaemonHandle,
    pub index: Arc<RwLock<FeatureIndex>>,
    pub timeline: Arc<TimelineStore>,
}

/// Tauri managed state.
///
/// Locking rules: `db` is a `std::sync::Mutex` — take it in a tight scope, never
/// across an `.await`; DB writes from async event tasks go through
/// `tokio::task::spawn_blocking`. The tokio mutexes are fine across awaits.
#[allow(dead_code)] // scaffold: consumed once command bodies are implemented
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    /// Open repos, keyed by canonical repo root.
    pub repos: tokio::sync::Mutex<HashMap<PathBuf, RepoRuntime>>,
    pub sessions: tokio::sync::Mutex<SessionManager>,
    /// Session id → repo root, for routing session events and cleanup.
    pub session_repos: tokio::sync::Mutex<HashMap<String, PathBuf>>,
}
