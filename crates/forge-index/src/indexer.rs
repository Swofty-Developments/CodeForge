//! Cold-start / doc-writing indexer driving headless `claude -p`.

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use forge_core::{Feature, IndexProgress};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinSet;

use crate::headless::run_headless_claude;
use crate::prompt::{decomposition_prompt, doc_prompt, DOC_MAX_LINES};
use crate::{parse, Result};

/// Max concurrent per-feature doc passes.
const DOC_CONCURRENCY: usize = 3;
const DOCS_DIR: &str = ".featureforge/docs";

/// Cold-start / incremental repository indexer.
///
/// Spawns headless `claude -p` (cwd = repo, `--output-format json`, sonnet) with a
/// decomposition prompt, validates the strict JSON feature array, clamps paths to
/// files that exist, then runs a parallel (capped) per-feature doc pass.
pub struct Indexer;

impl Indexer {
    /// Run the cold-start decomposition pass. Progress is streamed on
    /// `progress_tx` (stages: `scan` → `decompose` → `validate`); the caller then
    /// runs [`Self::write_feature_docs`] (stages: `docs` i-of-n → `done`) and
    /// forwards everything to the frontend as `index:progress` events. Returns the
    /// validated features (caller merges into [`crate::FeatureIndex`], preserving
    /// pinned entries).
    pub async fn cold_start(
        repo_root: &Path,
        progress_tx: mpsc::Sender<IndexProgress>,
    ) -> Result<Vec<Feature>> {
        progress(&progress_tx, "scan", "reading repository", 0, 0).await;

        progress(&progress_tx, "decompose", "mapping features with claude", 0, 0).await;
        let reply = run_headless_claude(repo_root, &decomposition_prompt()).await?;
        let raw = parse::parse_features(&reply)?;

        progress(
            &progress_tx,
            "validate",
            &format!("validating {} candidate features", raw.len()),
            0,
            raw.len() as u32,
        )
        .await;
        let features = parse::validate_features(repo_root, raw)?;
        progress(
            &progress_tx,
            "validate",
            &format!("{} features kept", features.len()),
            features.len() as u32,
            features.len() as u32,
        )
        .await;

        Ok(features)
    }

    /// Write/update `.featureforge/docs/<slug>.md` for each feature via a
    /// concurrency-capped headless claude doc pass. This crate (not claude) writes
    /// the returned markdown to disk. Pinned features are skipped so human-edited
    /// docs survive verbatim. Emits `docs` progress per feature, then `done`.
    pub async fn write_feature_docs(
        repo_root: &Path,
        features: &[Feature],
        progress_tx: mpsc::Sender<IndexProgress>,
    ) -> Result<()> {
        let docs_dir = repo_root.join(DOCS_DIR);
        tokio::fs::create_dir_all(&docs_dir).await?;

        let targets: Vec<Feature> = features.iter().filter(|f| !f.pinned).cloned().collect();
        let total = targets.len() as u32;
        if total == 0 {
            progress(&progress_tx, "done", "no docs to write", 0, 0).await;
            return Ok(());
        }

        let sem = Arc::new(Semaphore::new(DOC_CONCURRENCY));
        let done = Arc::new(AtomicU32::new(0));
        let mut set = JoinSet::new();
        for feature in targets {
            let sem = Arc::clone(&sem);
            let done = Arc::clone(&done);
            let tx = progress_tx.clone();
            let root = repo_root.to_path_buf();
            let docs_dir = docs_dir.clone();
            set.spawn(async move {
                let _permit = sem.acquire_owned().await;
                if let Err(e) = write_one_doc(&root, &docs_dir, &feature).await {
                    tracing::warn!(slug = %feature.slug, error = %e, "feature doc generation failed");
                }
                let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                progress(&tx, "docs", &feature.slug, n, total).await;
            });
        }
        while set.join_next().await.is_some() {}

        progress(&progress_tx, "done", "docs complete", total, total).await;
        Ok(())
    }
}

/// Generate one feature's markdown via headless claude and write it to disk.
async fn write_one_doc(repo_root: &Path, docs_dir: &Path, feature: &Feature) -> Result<()> {
    let reply = run_headless_claude(repo_root, &doc_prompt(feature)).await?;
    let body = clamp_lines(parse::strip_fences(&reply), DOC_MAX_LINES);
    let path = docs_dir.join(format!("{}.md", feature.slug));
    let tmp = path.with_extension("md.tmp");
    tokio::fs::write(&tmp, body.as_bytes()).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
}

async fn progress(
    tx: &mpsc::Sender<IndexProgress>,
    stage: &str,
    detail: &str,
    done: u32,
    total: u32,
) {
    // A dropped receiver (frontend closed) is not an error — indexing continues.
    let _ = tx
        .send(IndexProgress {
            stage: stage.to_string(),
            detail: detail.to_string(),
            done,
            total,
        })
        .await;
}

/// First-N-lines clamp with a guaranteed trailing newline.
fn clamp_lines(text: &str, max: usize) -> String {
    let mut out = text.lines().take(max).collect::<Vec<_>>().join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_lines_caps_and_terminates() {
        let text = (0..100).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
        let out = clamp_lines(&text, DOC_MAX_LINES);
        assert_eq!(out.lines().count(), DOC_MAX_LINES);
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn clamp_lines_short_input_untouched_but_terminated() {
        let out = clamp_lines("# Title\n\nbody", DOC_MAX_LINES);
        assert_eq!(out, "# Title\n\nbody\n");
    }

    /// Live cold-start against the real `claude` CLI on a tiny fixture repo.
    /// Run with `cargo test -p forge-index -- --ignored` (needs claude installed
    /// & authenticated).
    #[tokio::test]
    #[ignore = "spawns a real `claude` process"]
    async fn cold_start_on_tiny_fixture() {
        let tmp = crate::testutil::TempDir::new("coldstart");
        tmp.write(
            "Cargo.toml",
            "[package]\nname = \"tiny\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        tmp.write(
            "src/main.rs",
            "mod greet;\nfn main() { println!(\"{}\", greet::hello(\"world\")); }\n",
        );
        tmp.write(
            "src/greet.rs",
            "pub fn hello(name: &str) -> String { format!(\"hello, {name}\") }\n",
        );
        tmp.write("README.md", "# tiny\n\nA tiny greeter CLI.\n");

        let (tx, mut rx) = mpsc::channel(64);
        let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let features = Indexer::cold_start(tmp.path(), tx)
            .await
            .expect("cold_start should succeed");
        assert!(!features.is_empty(), "expected at least one feature");
        for f in &features {
            assert!(!f.slug.is_empty());
            assert!((0.0..=1.0).contains(&f.confidence));
            assert!(!f.entry_points.is_empty() || !f.files.is_empty());
        }

        Indexer::write_feature_docs(tmp.path(), &features, mpsc::channel(64).0)
            .await
            .expect("write_feature_docs should succeed");
        let doc = tmp
            .path()
            .join(DOCS_DIR)
            .join(format!("{}.md", features[0].slug));
        assert!(doc.is_file(), "expected a doc for the first feature");

        drain.await.unwrap();
    }
}
