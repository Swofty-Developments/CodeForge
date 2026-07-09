//! forge-daemon — per-repo local HTTP daemon (hooks receiver + query API),
//! integration-kit installer, and the `forge-mcp` stdio proxy binary.
//!
//! The daemon runs as an in-process tokio task inside the Tauri app but is
//! reachable externally: axum bound to `127.0.0.1:0`, actual port written to
//! `<repo>/.featureforge/runtime/daemon.json` (`{port, pid, started_at}`).

#![allow(dead_code)] // scaffold: fields are consumed once bodies are implemented

mod http;
mod kit;

pub use http::build_router;
pub use kit::{install_kit, KitReport};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use forge_index::FeatureIndex;
use forge_timeline::TimelineStore;
use tokio::sync::{oneshot, RwLock};

/// Daemon errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("timeline error: {0}")]
    Timeline(#[from] forge_timeline::Error),
    #[error("index error: {0}")]
    Index(#[from] forge_index::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Shared services the daemon serves from (owned by the Tauri app's RepoRuntime).
#[derive(Clone)]
pub struct DaemonDeps {
    pub repo_root: PathBuf,
    pub index: Arc<RwLock<FeatureIndex>>,
    pub timeline: Arc<TimelineStore>,
}

/// Handle to a running daemon. Dropping does not stop it; call [`Self::shutdown`].
pub struct DaemonHandle {
    pub port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl DaemonHandle {
    /// Stop the daemon and remove `daemon.json`.
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

/// Per-repo daemon.
pub struct Daemon;

impl Daemon {
    /// Bind axum to `127.0.0.1:0`, serve [`build_router`], write
    /// `.featureforge/runtime/daemon.json`, and return the handle with the
    /// actual port.
    pub async fn start(repo_root: &Path, deps: DaemonDeps) -> Result<DaemonHandle> {
        let _ = (repo_root, deps);
        // IMPLEMENT(agent): TcpListener::bind("127.0.0.1:0"), spawn
        // axum::serve(listener, build_router(deps)) with oneshot shutdown,
        // write daemon.json {port, pid, started_at}.
        todo!("Daemon::start")
    }
}
