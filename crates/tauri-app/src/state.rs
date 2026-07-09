use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use forge_daemon::DaemonHandle;
use forge_index::FeatureIndex;
use forge_session::SessionManager;
use forge_timeline::TimelineStore;
use tokio::sync::RwLock;

use crate::db::Database;

/// Everything running for one open repository.
pub struct RepoRuntime {
    pub daemon: DaemonHandle,
    pub index: Arc<RwLock<FeatureIndex>>,
    pub timeline: Arc<TimelineStore>,
    /// `repos.id` row for this repo (for `indexed_at` / thread inserts).
    pub repo_id: String,
    /// `timeline.subscribe()` → `timeline:event` forwarder; aborted on close.
    pub forwarder_task: tokio::task::JoinHandle<()>,
    /// True while a reindex task holds the slot (one reindex per repo).
    pub reindexing: Arc<AtomicBool>,
}

/// Tauri managed state.
///
/// Locking rules: `db` is a `std::sync::Mutex` — take it in a tight scope, never
/// across an `.await`; DB writes from async event tasks go through
/// `tokio::task::spawn_blocking`. The tokio mutexes are fine across awaits.
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    /// Open repos, keyed by canonical repo root.
    pub repos: tokio::sync::Mutex<HashMap<PathBuf, RepoRuntime>>,
    pub sessions: tokio::sync::Mutex<SessionManager>,
    /// Session id → repo root. Read on `close_repo` to tear down every session
    /// rooted at the closing repo (a session must not outlive its repo). NOT
    /// used for event routing — that is by `sessionId` in the payload (CONTRACT-1).
    pub session_repos: tokio::sync::Mutex<HashMap<String, PathBuf>>,
}

impl AppState {
    /// Clone the feature index handle for an open repo.
    pub async fn index(&self, key: &Path) -> Option<Arc<RwLock<FeatureIndex>>> {
        self.repos.lock().await.get(key).map(|rt| rt.index.clone())
    }

    /// Clone the timeline handle for an open repo.
    pub async fn timeline(&self, key: &Path) -> Option<Arc<TimelineStore>> {
        self.repos.lock().await.get(key).map(|rt| rt.timeline.clone())
    }

    /// Clone both the index and timeline handles for an open repo.
    pub async fn index_and_timeline(
        &self,
        key: &Path,
    ) -> Option<(Arc<RwLock<FeatureIndex>>, Arc<TimelineStore>)> {
        self.repos
            .lock()
            .await
            .get(key)
            .map(|rt| (rt.index.clone(), rt.timeline.clone()))
    }
}
