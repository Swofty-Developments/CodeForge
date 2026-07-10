//! Index metadata: a content-hash manifest written next to `features.json`, and
//! the pure staleness/version verdict computed from it (FZ-2).
//!
//! `.codeforge/index-meta.json` captures, at successful-index time, the
//! sha256 of every file referenced by any feature. [`status`] recomputes those
//! hashes to decide whether the on-disk index is still trustworthy. It is pure —
//! no `claude`, no re-index — so the UI can prompt a re-index without side
//! effects.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use forge_core::Feature;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Error, Result, FEATURES_FILE, INDEX_VERSION};

/// Meta file location, relative to the repo root.
pub(crate) const META_FILE: &str = ".codeforge/index-meta.json";

/// On-disk manifest: the index-format version plus the sha256 (hex) of every
/// referenced file's contents at index time. `files` is keyed by repo-relative
/// path with `/` separators for a stable, platform-independent JSON shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexMeta {
    pub version: u32,
    pub files: BTreeMap<String, String>,
}

/// Distinct, named staleness/version state of an on-disk index. Serialized as a
/// lowercase string so the frontend sees the `"never" | "fresh" | "stale" |
/// "outdated"` union. Never conflated — there is no implicit fallback verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexState {
    /// No index yet (features.json or index-meta.json is absent).
    Never,
    /// Stored version + every manifest hash match: the index is trustworthy.
    Fresh,
    /// Some manifest files' content changed (or vanished) since indexing —
    /// edited outside our hooks (e.g. a `git pull` by a non-CodeForge user).
    Stale,
    /// The stored index-format version predates the current [`INDEX_VERSION`].
    Outdated,
}

/// The staleness verdict surfaced to the UI (FZ-2). `state` is the single named
/// verdict; `changedFiles` is only populated for [`IndexState::Stale`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub state: IndexState,
    /// Repo-relative manifest paths whose current sha256 differs from the stored
    /// one, or that no longer exist. Empty except when `state` is `stale`.
    pub changed_files: Vec<String>,
    /// Version recorded in the on-disk meta (0 when there is no meta yet).
    pub index_version: u32,
    /// The indexer's current [`INDEX_VERSION`].
    pub current_version: u32,
}

impl IndexStatus {
    /// No index on disk yet.
    fn never() -> Self {
        Self {
            state: IndexState::Never,
            changed_files: Vec::new(),
            index_version: 0,
            current_version: INDEX_VERSION,
        }
    }
}

/// Write `.codeforge/index-meta.json` for the current feature set: hash the
/// contents of every referenced file that exists and record the current
/// [`INDEX_VERSION`]. Atomic (tmp + rename) so a reader never sees a half file.
///
/// Rule for a referenced path that does not exist at write time: it is a path
/// with no content to hash, so it is omitted from the manifest (logged). It is
/// therefore not tracked for staleness — a deliberate, documented rule, not a
/// silent guess.
pub(crate) fn write(repo_root: &Path, features: &[Feature]) -> Result<()> {
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    for rel in manifest_paths(features) {
        match hash_file(&repo_root.join(&rel)) {
            Some(hash) => {
                files.insert(rel, hash);
            }
            None => tracing::warn!(
                path = %rel,
                "index-meta: referenced file missing at write, omitting from manifest"
            ),
        }
    }

    let meta = IndexMeta { version: INDEX_VERSION, files };
    let path = repo_root.join(META_FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(&meta)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, format!("{json}\n"))?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Compute the on-disk index staleness/version verdict (FZ-2). Pure: reads
/// `features.json` + `index-meta.json` and re-hashes the manifest files.
///
/// - `features.json` OR `index-meta.json` absent → [`IndexState::Never`].
/// - stored `version` < [`INDEX_VERSION`] → [`IndexState::Outdated`].
/// - a manifest file's content changed or it vanished → [`IndexState::Stale`]
///   (with the changed paths); otherwise → [`IndexState::Fresh`].
pub(crate) fn status(repo_root: &Path) -> Result<IndexStatus> {
    if !repo_root.join(FEATURES_FILE).exists() || !repo_root.join(META_FILE).exists() {
        return Ok(IndexStatus::never());
    }

    let text = std::fs::read_to_string(repo_root.join(META_FILE))?;
    let meta: IndexMeta = serde_json::from_str(&text).map_err(Error::Json)?;

    if meta.version < INDEX_VERSION {
        return Ok(IndexStatus {
            state: IndexState::Outdated,
            changed_files: Vec::new(),
            index_version: meta.version,
            current_version: INDEX_VERSION,
        });
    }

    let mut changed: Vec<String> = Vec::new();
    for (rel, expected) in &meta.files {
        match hash_file(&repo_root.join(rel)) {
            Some(actual) if &actual == expected => {}
            // Changed content OR a file that has since vanished — both mean the
            // stored index no longer matches the tree.
            _ => changed.push(rel.clone()),
        }
    }
    changed.sort();

    let state = if changed.is_empty() { IndexState::Fresh } else { IndexState::Stale };
    Ok(IndexStatus {
        state,
        changed_files: changed,
        index_version: meta.version,
        current_version: INDEX_VERSION,
    })
}

/// Every repo-relative path referenced by any feature (entry_points ∪ files),
/// deduplicated and sorted, with `/` separators.
fn manifest_paths(features: &[Feature]) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    for feature in features {
        for path in &feature.entry_points {
            set.insert(path_key(path));
        }
        for file in &feature.files {
            set.insert(path_key(&file.path));
        }
    }
    set.into_iter().collect()
}

/// Stable manifest key: repo-relative path with `/` separators.
fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// sha256 (hex) of a file's contents, or `None` if it cannot be read.
fn hash_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use forge_core::{FeatureFile, FileRole};

    use super::*;
    use crate::testutil::TempDir;

    fn feature(slug: &str, files: &[&str]) -> Feature {
        Feature {
            slug: slug.into(),
            name: slug.into(),
            description: String::new(),
            entry_points: Vec::new(),
            files: files
                .iter()
                .map(|p| FeatureFile { path: PathBuf::from(p), role: FileRole::Core, pinned: false })
                .collect(),
            tags: Vec::new(),
            pinned: false,
            color: None,
            group: None,
            updated_at: Utc::now(),
        }
    }

    /// Write features.json + meta for a fixture so `status` has both anchors.
    fn seed(tmp: &TempDir, features: &[Feature]) {
        tmp.write(FEATURES_FILE, &serde_json::to_string(features).unwrap());
        write(tmp.path(), features).expect("write meta");
    }

    #[test]
    fn never_without_features_or_meta() {
        let tmp = TempDir::new("never");
        let st = status(tmp.path()).expect("status");
        assert_eq!(st.state, IndexState::Never);
        assert_eq!(st.index_version, 0);
        assert_eq!(st.current_version, INDEX_VERSION);

        // features.json present but meta absent is still "never".
        tmp.write(FEATURES_FILE, "[]");
        assert_eq!(status(tmp.path()).unwrap().state, IndexState::Never);
    }

    #[test]
    fn fresh_when_hashes_and_version_match() {
        let tmp = TempDir::new("fresh");
        tmp.write("src/a.rs", "fn a() {}");
        seed(&tmp, &[feature("a", &["src/a.rs"])]);
        let st = status(tmp.path()).expect("status");
        assert_eq!(st.state, IndexState::Fresh);
        assert!(st.changed_files.is_empty());
        assert_eq!(st.index_version, INDEX_VERSION);
    }

    #[test]
    fn stale_when_a_file_content_changes() {
        let tmp = TempDir::new("stale");
        tmp.write("src/a.rs", "fn a() {}");
        tmp.write("src/b.rs", "fn b() {}");
        seed(&tmp, &[feature("a", &["src/a.rs", "src/b.rs"])]);

        // Edit one file outside our hooks (real sha256 must differ).
        tmp.write("src/b.rs", "fn b() { changed(); }");
        let st = status(tmp.path()).expect("status");
        assert_eq!(st.state, IndexState::Stale);
        assert_eq!(st.changed_files, vec!["src/b.rs".to_string()]);
    }

    #[test]
    fn stale_when_a_manifest_file_vanishes() {
        let tmp = TempDir::new("vanish");
        tmp.write("src/a.rs", "fn a() {}");
        seed(&tmp, &[feature("a", &["src/a.rs"])]);
        std::fs::remove_file(tmp.path().join("src/a.rs")).unwrap();
        let st = status(tmp.path()).expect("status");
        assert_eq!(st.state, IndexState::Stale);
        assert_eq!(st.changed_files, vec!["src/a.rs".to_string()]);
    }

    #[test]
    fn outdated_when_stored_version_is_older() {
        let tmp = TempDir::new("outdated");
        tmp.write("src/a.rs", "fn a() {}");
        seed(&tmp, &[feature("a", &["src/a.rs"])]);

        // Rewrite meta with an older version; even matching hashes lose to it.
        let mut meta: IndexMeta =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join(META_FILE)).unwrap())
                .unwrap();
        meta.version = INDEX_VERSION.saturating_sub(1);
        tmp.write(META_FILE, &serde_json::to_string_pretty(&meta).unwrap());

        let st = status(tmp.path()).expect("status");
        assert_eq!(st.state, IndexState::Outdated);
        assert_eq!(st.index_version, INDEX_VERSION.saturating_sub(1));
        assert_eq!(st.current_version, INDEX_VERSION);
    }

    #[test]
    fn index_status_serializes_to_the_frozen_ipc_shape() {
        // FZ-2 contract: { state, changedFiles, indexVersion, currentVersion },
        // state a lowercase string union. Mirrored in frontend/src/types.ts.
        let st = IndexStatus {
            state: IndexState::Stale,
            changed_files: vec!["src/b.rs".into()],
            index_version: 1,
            current_version: 1,
        };
        let json = serde_json::to_string(&st).unwrap();
        assert!(json.contains("\"state\":\"stale\""));
        assert!(json.contains("\"changedFiles\""));
        assert!(json.contains("\"indexVersion\""));
        assert!(json.contains("\"currentVersion\""));
        assert!(!json.contains("changed_files"));
        assert_eq!(serde_json::to_string(&IndexState::Never).unwrap(), "\"never\"");
        assert_eq!(serde_json::to_string(&IndexState::Outdated).unwrap(), "\"outdated\"");
    }

    #[test]
    fn manifest_dedups_entry_points_and_files() {
        let mut f = feature("x", &["src/a.rs", "src/b.rs"]);
        f.entry_points = vec![PathBuf::from("src/a.rs")]; // also an entry point
        let paths = manifest_paths(&[f]);
        assert_eq!(paths, vec!["src/a.rs".to_string(), "src/b.rs".to_string()]);
    }
}
