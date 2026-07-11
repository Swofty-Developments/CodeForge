//! forge-index — the feature model: load/save, pins, path→feature classifier,
//! and the headless-`claude` cold-start indexer.
//!
//! Storage (per repo, committable): `.codeforge/features.json` (human-readable
//! JSON array of [`Feature`]) and `.codeforge/docs/<slug>.md` (living docs).

mod classify;
mod headless;
mod indexer;
mod merge;
mod meta;
mod parse;
mod prompt;
mod refresh;

pub use indexer::{DocReport, Indexer};
pub use refresh::refresh_feature_doc;
pub use meta::{IndexMeta, IndexState, IndexStatus};

use std::path::{Path, PathBuf};

use chrono::Utc;
use forge_core::{Feature, FeaturePatch};

/// Index-format version. Bump when the indexing *technique* changes (prompt,
/// derivation, meta shape) so a previously-written index reports `outdated` and
/// the UI can prompt a re-index (FZ-2). Stored in `index-meta.json`.
/// v2: granularity + multi-level group prompt, deep derive_group rule.
pub const INDEX_VERSION: u32 = 2;

/// Pure staleness/version verdict for the on-disk index at `repo_root` (FZ-2).
/// Reads `features.json` + `index-meta.json` and re-hashes the manifest files;
/// never runs `claude` and never re-indexes.
pub fn index_status(repo_root: &Path) -> Result<IndexStatus> {
    meta::status(repo_root)
}

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

/// Index file location, relative to the repo root.
pub(crate) const FEATURES_FILE: &str = ".codeforge/features.json";

/// In-memory feature index for one repo, backed by `.codeforge/features.json`.
pub struct FeatureIndex {
    repo_root: PathBuf,
    features: Vec<Feature>,
}

impl FeatureIndex {
    /// Load `<repo_root>/.codeforge/features.json`. A missing file yields an
    /// empty index (fresh repo, pre-index).
    pub fn load(repo_root: &Path) -> Result<Self> {
        let path = repo_root.join(FEATURES_FILE);
        let features = match std::fs::read_to_string(&path) {
            Ok(text) => {
                let mut features: Vec<Feature> = serde_json::from_str(&text)?;
                features.sort_by(|a, b| a.slug.cmp(&b.slug));
                features
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self { repo_root: repo_root.to_path_buf(), features })
    }

    /// Persist the index back to `features.json` (pretty-printed, stable slug order).
    pub fn save(&self) -> Result<()> {
        let path = self.repo_root.join(FEATURES_FILE);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut sorted: Vec<&Feature> = self.features.iter().collect();
        sorted.sort_by(|a, b| a.slug.cmp(&b.slug));
        let json = serde_json::to_string_pretty(&sorted)?;
        // Atomic write: readers never observe a half-written index.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, format!("{json}\n"))?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Rewrite `.codeforge/index-meta.json` from the current feature set:
    /// the content-hash manifest + the current [`INDEX_VERSION`] (FZ-2). Call
    /// after a successful index/save so [`index_status`] returns `fresh`.
    pub fn write_meta(&self) -> Result<()> {
        meta::write(&self.repo_root, &self.features)
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
    /// re-index merges (caller-side contract; use [`Self::merge_reindex`]).
    pub fn upsert(&mut self, mut feature: Feature) {
        feature.updated_at = Utc::now();
        match self.features.iter_mut().find(|f| f.slug == feature.slug) {
            Some(slot) => *slot = feature,
            None => {
                self.features.push(feature);
                self.features.sort_by(|a, b| a.slug.cmp(&b.slug));
            }
        }
    }

    /// Merge a re-index result into the index (product-critical semantics):
    /// pinned features survive verbatim; pinned files inside unpinned features
    /// survive the file-list replacement; unpinned features are replaced by slug
    /// (dropped when absent from `incoming`); new slugs are appended.
    /// Purely in-memory — `docs/<slug>.md` files are never deleted.
    pub fn merge_reindex(&mut self, incoming: Vec<Feature>) {
        self.features = merge::merge(std::mem::take(&mut self.features), incoming);
    }

    /// Pin/unpin a feature. Errors with [`Error::NotFound`] on unknown slug.
    pub fn pin(&mut self, slug: &str, pinned: bool) -> Result<()> {
        let feature = self
            .features
            .iter_mut()
            .find(|f| f.slug == slug)
            .ok_or_else(|| Error::NotFound(slug.to_string()))?;
        feature.pinned = pinned;
        feature.updated_at = Utc::now();
        Ok(())
    }

    /// Apply a human edit; returns the updated feature. Editing implies pinning.
    pub fn apply_patch(&mut self, slug: &str, patch: FeaturePatch) -> Result<Feature> {
        let feature = self
            .features
            .iter_mut()
            .find(|f| f.slug == slug)
            .ok_or_else(|| Error::NotFound(slug.to_string()))?;
        if let Some(name) = patch.name {
            feature.name = name;
        }
        if let Some(description) = patch.description {
            feature.description = description;
        }
        if let Some(tags) = patch.tags {
            feature.tags = tags;
        }
        feature.pinned = true;
        feature.updated_at = Utc::now();
        Ok(feature.clone())
    }

    /// Map changed paths to the slugs of the features they belong to
    /// (exact file match, then nearest-directory match). Deduplicated.
    /// Paths matching nothing are omitted — callers queue them for re-index.
    pub fn classify_paths(&self, paths: &[PathBuf]) -> Vec<String> {
        classify::classify(&self.repo_root, &self.features, paths)
    }

    /// Slugs of features that reference any of `rel_paths` (repo-relative,
    /// `/`-separated — the index-meta manifest keys) as an entry point or file.
    /// Exact membership only; no directory-prefix widening.
    pub fn owners_of(&self, rel_paths: &[String]) -> Vec<String> {
        let set: std::collections::BTreeSet<&str> =
            rel_paths.iter().map(String::as_str).collect();
        self.features
            .iter()
            .filter(|f| {
                f.entry_points
                    .iter()
                    .chain(f.files.iter().map(|file| &file.path))
                    .any(|p| set.contains(p.to_string_lossy().replace('\\', "/").as_str()))
            })
            .map(|f| f.slug.clone())
            .collect()
    }

    /// Drop references to files that no longer exist on disk from the named
    /// features. An unpinned feature left with no paths at all is removed (its
    /// subject matter is gone); pinned features survive minus the dead paths.
    /// Returns the slugs of removed features.
    pub fn prune_missing_files(&mut self, slugs: &[String]) -> Vec<String> {
        let root = self.repo_root.clone();
        for feature in self.features.iter_mut().filter(|f| slugs.contains(&f.slug)) {
            let before = feature.entry_points.len() + feature.files.len();
            feature.entry_points.retain(|p| root.join(p).exists());
            feature.files.retain(|f| root.join(&f.path).exists());
            if feature.entry_points.len() + feature.files.len() != before {
                feature.updated_at = Utc::now();
            }
        }
        let mut removed = Vec::new();
        self.features.retain(|f| {
            let dead = !f.pinned
                && slugs.contains(&f.slug)
                && f.entry_points.is_empty()
                && f.files.is_empty();
            if dead {
                removed.push(f.slug.clone());
            }
            !dead
        });
        removed
    }

    /// The repo this index belongs to.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Read the living doc `.codeforge/docs/<slug>.md`, if one has been
    /// written. Returns `None` when the feature has no doc yet (pre-index).
    pub fn read_doc(&self, slug: &str) -> Result<Option<String>> {
        let path = self.repo_root.join(".codeforge/docs").join(format!("{slug}.md"));
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
pub(crate) mod testutil {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Self-cleaning temp dir (keeps the crate free of a tempfile dependency).
    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "forge-index-{label}-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&dir).expect("create temp dir");
            Self(dir)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }

        pub fn write(&self, rel: &str, contents: &str) {
            let path = self.0.join(rel);
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).expect("create parent dirs");
            }
            std::fs::write(path, contents).expect("write fixture file");
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use forge_core::{FeatureFile, FileRole};

    use super::testutil::TempDir;
    use super::*;

    fn feature(slug: &str, pinned: bool) -> Feature {
        Feature {
            slug: slug.into(),
            name: slug.to_uppercase(),
            description: String::new(),
            entry_points: Vec::new(),
            files: vec![FeatureFile {
                path: PathBuf::from(format!("src/{slug}.rs")),
                role: FileRole::Core,
                pinned: false,
            }],
            tags: Vec::new(),
            pinned,
            color: None,
            group: None,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn save_then_load_round_trips_in_stable_slug_order() {
        let tmp = TempDir::new("roundtrip");
        let mut index = FeatureIndex::load(tmp.path()).expect("empty load");
        assert!(index.features().is_empty());

        index.upsert(feature("beta", false));
        index.upsert(feature("alpha", false));
        index.save().expect("save");

        // On-disk JSON is pretty and slug-sorted with a trailing newline.
        let raw = std::fs::read_to_string(tmp.path().join(FEATURES_FILE)).expect("read");
        assert!(raw.ends_with("\n"));
        assert!(raw.contains("\"slug\": \"alpha\""));
        assert!(raw.find("\"alpha\"").unwrap() < raw.find("\"beta\"").unwrap());

        let reloaded = FeatureIndex::load(tmp.path()).expect("reload");
        let slugs: Vec<_> = reloaded.features().iter().map(|f| f.slug.clone()).collect();
        assert_eq!(slugs, vec!["alpha", "beta"]);
    }

    #[test]
    fn pin_and_patch_report_missing_slugs() {
        let tmp = TempDir::new("missing");
        let mut index = FeatureIndex::load(tmp.path()).expect("load");
        assert!(matches!(index.pin("ghost", true), Err(Error::NotFound(_))));
        assert!(matches!(
            index.apply_patch("ghost", FeaturePatch::default()),
            Err(Error::NotFound(_))
        ));
    }

    #[test]
    fn apply_patch_updates_fields_and_pins() {
        let tmp = TempDir::new("patch");
        let mut index = FeatureIndex::load(tmp.path()).expect("load");
        index.upsert(feature("auth", false));

        let patched = index
            .apply_patch(
                "auth",
                FeaturePatch {
                    name: Some("Renamed".into()),
                    description: Some("new desc".into()),
                    tags: Some(vec!["security".into()]),
                },
            )
            .expect("patch");
        assert_eq!(patched.name, "Renamed");
        assert_eq!(patched.description, "new desc");
        assert_eq!(patched.tags, vec!["security".to_string()]);
        assert!(patched.pinned, "editing implies pinning");
        assert!(index.get("auth").unwrap().pinned);
    }

    #[test]
    fn merge_reindex_preserves_group_on_both_paths() {
        let tmp = TempDir::new("group-merge");
        let mut index = FeatureIndex::load(tmp.path()).expect("load");

        // A pinned feature with a hand-edited group, and an unpinned one.
        let mut pinned = feature("pinned", true);
        pinned.group = Some("hand/edited".into());
        let mut unpinned = feature("unpinned", false);
        unpinned.group = Some("old".into());
        index.upsert(pinned);
        index.upsert(unpinned);

        // Re-index: the pinned feature's incoming group must be ignored; the
        // unpinned feature adopts the fresh group.
        let mut fresh_pinned = feature("pinned", false);
        fresh_pinned.group = Some("SHOULD NOT WIN".into());
        let mut fresh_unpinned = feature("unpinned", false);
        fresh_unpinned.group = Some("crates/forge-index".into());
        index.merge_reindex(vec![fresh_pinned, fresh_unpinned]);

        assert_eq!(index.get("pinned").unwrap().group.as_deref(), Some("hand/edited"));
        assert_eq!(index.get("unpinned").unwrap().group.as_deref(), Some("crates/forge-index"));
    }

    #[test]
    fn owners_of_matches_exact_membership_only() {
        let tmp = TempDir::new("owners");
        let mut index = FeatureIndex::load(tmp.path()).expect("load");
        index.upsert(feature("auth", false)); // references src/auth.rs
        index.upsert(feature("billing", false)); // references src/billing.rs

        assert_eq!(index.owners_of(&["src/auth.rs".to_string()]), vec!["auth"]);
        // A sibling under src/ is NOT an owner — no directory-prefix widening.
        assert!(index.owners_of(&["src/other.rs".to_string()]).is_empty());
    }

    #[test]
    fn prune_missing_files_drops_dead_paths_and_empty_unpinned_features() {
        let tmp = TempDir::new("prune");
        tmp.write("src/kept.rs", "fn kept() {}");
        let mut index = FeatureIndex::load(tmp.path()).expect("load");

        // "kept" has one real file; "gone" (unpinned) and "gone-pinned" (pinned)
        // reference only files that don't exist on disk.
        index.upsert(feature("kept", false));
        index.get("kept").unwrap(); // sanity
        index.upsert(feature("gone", false));
        index.upsert(feature("gone-pinned", true));
        // Point "kept" at the real file.
        let mut kept = index.get("kept").unwrap().clone();
        kept.files[0].path = PathBuf::from("src/kept.rs");
        index.upsert(kept);

        let slugs: Vec<String> =
            ["kept", "gone", "gone-pinned"].iter().map(|s| s.to_string()).collect();
        let removed = index.prune_missing_files(&slugs);

        assert_eq!(removed, vec!["gone".to_string()]);
        assert!(index.get("kept").is_some(), "feature with a live file survives");
        let pinned = index.get("gone-pinned").expect("pinned survives even when empty");
        assert!(pinned.files.is_empty(), "dead paths pruned from pinned feature");
    }

    #[test]
    fn merge_reindex_preserves_pinned_feature_through_public_api() {
        let tmp = TempDir::new("reindex");
        let mut index = FeatureIndex::load(tmp.path()).expect("load");
        index.upsert(feature("kept", true));
        index.upsert(feature("stale", false));

        // Re-index returns a fresh "kept" plus a brand-new "new"; "stale" vanishes.
        let mut fresh_kept = feature("kept", false);
        fresh_kept.name = "SHOULD NOT WIN".into();
        index.merge_reindex(vec![fresh_kept, feature("new", false)]);

        let slugs: Vec<_> = index.features().iter().map(|f| f.slug.clone()).collect();
        assert_eq!(slugs, vec!["kept", "new"]);
        assert_eq!(index.get("kept").unwrap().name, "KEPT"); // pinned verbatim
        assert!(index.get("stale").is_none());
    }
}
