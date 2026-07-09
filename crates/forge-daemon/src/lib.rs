//! forge-daemon — per-repo local HTTP daemon (hooks receiver + query API),
//! integration-kit installer, and the `forge-mcp` stdio proxy binary.
//!
//! The daemon runs as an in-process tokio task inside the Tauri app but is
//! reachable externally: axum bound to `127.0.0.1:0`, actual port written to
//! `<repo>/.featureforge/runtime/daemon.json` (`{port, pid, started_at}`).

mod hooks;
mod http;
mod ingest;
mod kit;
mod kit_assets;
mod reindex_queue;

pub use http::build_router;
pub use kit::{install_kit, KitReport};
pub use reindex_queue::{PendingReindex, ReindexQueue};

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
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
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
        if let Some(mut task) = self.task.take() {
            // Graceful drain should be near-instant; abort as a backstop.
            if tokio::time::timeout(std::time::Duration::from_secs(5), &mut task)
                .await
                .is_err()
            {
                tracing::warn!("daemon did not shut down within 5s; aborting task");
                task.abort();
            }
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
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let started_at = chrono::Utc::now();

        let runtime_dir = repo_root.join(".featureforge").join("runtime");
        std::fs::create_dir_all(&runtime_dir)?;
        let daemon_json = runtime_dir.join("daemon.json");
        let manifest = serde_json::json!({
            "port": port,
            "pid": std::process::id(),
            "started_at": started_at.to_rfc3339(),
        });
        std::fs::write(&daemon_json, format!("{:#}\n", manifest))?;

        let reindex_queue = Arc::new(ReindexQueue::open(repo_root)?);
        // Replay hook payloads forward.sh spooled while the daemon was down,
        // before we start serving live ones (drain borrows deps; router moves it).
        ingest::drain_spool(&deps, &reindex_queue).await;
        let router = http::build_router_with_info(deps, port, started_at, reindex_queue);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let serve = axum::serve(listener, router).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });
            if let Err(e) = serve.await {
                tracing::error!("daemon serve error: {e}");
            }
            if let Err(e) = std::fs::remove_file(&daemon_json) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!("failed to remove daemon.json: {e}");
                }
            }
        });

        tracing::info!(port, "featureforge daemon listening");
        Ok(DaemonHandle {
            port,
            shutdown_tx: Some(shutdown_tx),
            task: Some(task),
        })
    }
}
