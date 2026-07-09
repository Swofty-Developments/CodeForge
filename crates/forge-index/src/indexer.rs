use std::path::Path;

use forge_core::{Feature, IndexProgress};
use tokio::sync::mpsc;

use crate::Result;

/// Cold-start / incremental repository indexer.
///
/// Spawns headless `claude -p` (cwd = repo, `--output-format json`, sonnet) with a
/// decomposition prompt, validates the strict JSON feature array, clamps paths to
/// files that exist, then runs a parallel (capped) per-feature doc pass.
pub struct Indexer;

impl Indexer {
    /// Run the cold-start decomposition pass. Progress is streamed on
    /// `progress_tx` (stages: "decompose", "docs"); the caller forwards it to the
    /// frontend as `index:progress` events. Returns the validated features
    /// (caller merges into [`crate::FeatureIndex`], preserving pinned entries).
    pub async fn cold_start(
        repo_root: &Path,
        progress_tx: mpsc::Sender<IndexProgress>,
    ) -> Result<Vec<Feature>> {
        let _ = (repo_root, progress_tx);
        // IMPLEMENT(agent): resolve `claude` via forge_session::shell_env::which,
        // spawn `claude -p --output-format json --model sonnet` with the
        // decomposition prompt, parse/validate/clamp, stream progress.
        todo!("Indexer::cold_start")
    }

    /// Write/update `.featureforge/docs/<slug>.md` for each feature (parallel,
    /// concurrency-capped headless claude doc pass). Existing human-edited
    /// (pinned) docs are left verbatim.
    pub async fn write_feature_docs(
        repo_root: &Path,
        features: &[Feature],
        progress_tx: mpsc::Sender<IndexProgress>,
    ) -> Result<()> {
        let _ = (repo_root, features, progress_tx);
        // IMPLEMENT(agent): per-feature prompt -> markdown doc; cap parallelism
        // (e.g. 4); emit IndexProgress{stage:"docs", detail:slug, done, total}.
        todo!("Indexer::write_feature_docs")
    }
}
