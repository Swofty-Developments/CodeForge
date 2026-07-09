//! forge-index — the feature model: load/save, pins, path→feature classifier,
//! and the headless-`claude` cold-start indexer.
//!
//! Storage (per repo, committable): `.featureforge/features.json` (human-readable
//! JSON array of [`Feature`]) and `.featureforge/docs/<slug>.md` (living docs).

#![allow(dead_code)] // scaffold: fields are consumed once bodies are implemented

mod indexer;

pub use indexer::Indexer;

use std::path::{Path, PathBuf};

use forge_core::{Feature, FeaturePatch};

/// Index errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("feature not found: {0}")]
    NotFound(String),
    #[error("indexer failed: {0}")]
    Indexer(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// In-memory feature index for one repo, backed by `.featureforge/features.json`.
pub struct FeatureIndex {
    repo_root: PathBuf,
    features: Vec<Feature>,
}

impl FeatureIndex {
    /// Load `<repo_root>/.featureforge/features.json`. A missing file yields an
    /// empty index (fresh repo, pre-index).
    pub fn load(repo_root: &Path) -> Result<Self> {
        let _ = repo_root;
        // IMPLEMENT(agent): read + parse features.json; missing file => empty vec.
        todo!("FeatureIndex::load")
    }

    /// Persist the index back to `features.json` (pretty-printed, stable slug order).
    pub fn save(&self) -> Result<()> {
        // IMPLEMENT(agent): atomic write (tmp + rename).
        todo!("FeatureIndex::save")
    }

    /// All features, sorted by slug.
    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    /// Look up one feature by slug.
    pub fn get(&self, slug: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.slug == slug)
    }

    /// Insert or replace by slug. Pinned features/files must be preserved by
    /// re-index merges (caller-side contract; see docs/ARCHITECTURE.md §Indexing).
    pub fn upsert(&mut self, feature: Feature) {
        let _ = feature;
        // IMPLEMENT(agent): replace by slug or push; keep slug-sorted; bump updated_at.
        todo!("FeatureIndex::upsert")
    }

    /// Pin/unpin a feature. Errors with [`Error::NotFound`] on unknown slug.
    pub fn pin(&mut self, slug: &str, pinned: bool) -> Result<()> {
        let _ = (slug, pinned);
        // IMPLEMENT(agent): set pinned + updated_at.
        todo!("FeatureIndex::pin")
    }

    /// Apply a human edit; returns the updated feature. Editing implies pinning.
    pub fn apply_patch(&mut self, slug: &str, patch: FeaturePatch) -> Result<Feature> {
        let _ = (slug, patch);
        // IMPLEMENT(agent): apply Some() fields, set pinned=true, bump updated_at.
        todo!("FeatureIndex::apply_patch")
    }

    /// Map changed paths to the slugs of the features they belong to
    /// (exact file match, then nearest-directory match). Deduplicated.
    /// Paths matching nothing are omitted — callers queue them for re-index.
    pub fn classify_paths(&self, paths: &[PathBuf]) -> Vec<String> {
        let _ = paths;
        // IMPLEMENT(agent): exact FeatureFile.path match; fallback to longest
        // shared directory prefix among feature files/entry points.
        todo!("FeatureIndex::classify_paths")
    }

    /// The repo this index belongs to.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}
