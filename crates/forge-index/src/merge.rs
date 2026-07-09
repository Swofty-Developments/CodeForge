//! Re-index merge semantics (product-critical, see docs/ARCHITECTURE.md §Indexing).

use std::collections::BTreeMap;

use chrono::Utc;
use forge_core::Feature;

/// Merge a fresh re-index result into the existing feature list:
/// - pinned existing features survive verbatim (incoming version discarded);
/// - unpinned existing features are replaced by slug, but their pinned
///   [`forge_core::FeatureFile`] entries survive the file-list replacement;
/// - unpinned existing slugs absent from `incoming` are dropped;
/// - new incoming slugs are appended. Result is slug-sorted.
pub(crate) fn merge(existing: Vec<Feature>, incoming: Vec<Feature>) -> Vec<Feature> {
    let mut incoming_by_slug: BTreeMap<String, Feature> =
        incoming.into_iter().map(|f| (f.slug.clone(), f)).collect();

    let mut merged: Vec<Feature> = Vec::new();
    for old in existing {
        if old.pinned {
            incoming_by_slug.remove(&old.slug);
            merged.push(old);
        } else if let Some(mut new) = incoming_by_slug.remove(&old.slug) {
            carry_pinned_files(&old, &mut new);
            new.updated_at = Utc::now();
            merged.push(new);
        } else {
            tracing::debug!(slug = %old.slug, "re-index dropped unpinned feature");
        }
    }

    merged.extend(incoming_by_slug.into_values());
    merged.sort_by(|a, b| a.slug.cmp(&b.slug));
    merged
}

/// Pinned file entries survive verbatim: same-path incoming entries are
/// overwritten with the human-blessed one, missing ones are re-appended.
fn carry_pinned_files(old: &Feature, new: &mut Feature) {
    for pinned_file in old.files.iter().filter(|f| f.pinned) {
        match new.files.iter_mut().find(|nf| nf.path == pinned_file.path) {
            Some(slot) => *slot = pinned_file.clone(),
            None => new.files.push(pinned_file.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use forge_core::{FeatureFile, FileRole};

    use super::*;

    fn file(path: &str, pinned: bool) -> FeatureFile {
        FeatureFile { path: PathBuf::from(path), role: FileRole::Core, pinned }
    }

    fn feature(slug: &str, name: &str, pinned: bool, files: Vec<FeatureFile>) -> Feature {
        Feature {
            slug: slug.into(),
            name: name.into(),
            description: String::new(),
            entry_points: Vec::new(),
            files,
            tags: Vec::new(),
            confidence: 0.5,
            pinned,
            color: None,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn pinned_feature_survives_verbatim() {
        let existing = vec![feature("auth", "Kept name", true, vec![file("a.rs", false)])];
        let incoming = vec![feature("auth", "New name", false, vec![file("b.rs", false)])];

        let merged = merge(existing, incoming);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "Kept name");
        assert!(merged[0].pinned);
        assert_eq!(merged[0].files, vec![file("a.rs", false)]);
    }

    #[test]
    fn pinned_file_survives_in_unpinned_feature() {
        let existing = vec![feature(
            "api",
            "Api",
            false,
            vec![file("keep.rs", true), file("drop.rs", false)],
        )];
        let incoming = vec![feature("api", "Api v2", false, vec![file("fresh.rs", false)])];

        let merged = merge(existing, incoming);
        assert_eq!(merged.len(), 1);
        let f = &merged[0];
        assert_eq!(f.name, "Api v2"); // unpinned feature body replaced
        let paths: Vec<_> = f.files.iter().map(|x| x.path.to_string_lossy().to_string()).collect();
        assert!(paths.contains(&"fresh.rs".to_string()));
        assert!(paths.contains(&"keep.rs".to_string())); // pinned file carried over
        assert!(!paths.contains(&"drop.rs".to_string())); // unpinned file dropped
        assert!(f.files.iter().find(|x| x.path.ends_with("keep.rs")).unwrap().pinned);
    }

    #[test]
    fn pinned_file_overwrites_same_path_incoming() {
        // Human pinned "shared.rs" as a Test; re-index calls it Core → pin wins.
        let mut pinned = file("shared.rs", true);
        pinned.role = FileRole::Test;
        let existing = vec![feature("f", "F", false, vec![pinned])];
        let incoming = vec![feature("f", "F", false, vec![file("shared.rs", false)])];

        let merged = merge(existing, incoming);
        assert_eq!(merged[0].files.len(), 1);
        assert_eq!(merged[0].files[0].role, FileRole::Test);
        assert!(merged[0].files[0].pinned);
    }

    #[test]
    fn unpinned_feature_absent_from_incoming_is_dropped() {
        let existing = vec![feature("stale", "Stale", false, vec![file("x.rs", false)])];
        let merged = merge(existing, Vec::new());
        assert!(merged.is_empty());
    }

    #[test]
    fn new_slugs_appended_and_result_sorted() {
        let existing = vec![feature("b", "B old", false, vec![])];
        let incoming = vec![
            feature("c", "C", false, vec![]),
            feature("a", "A", false, vec![]),
            feature("b", "B new", false, vec![]),
        ];
        let merged = merge(existing, incoming);
        let slugs: Vec<_> = merged.iter().map(|f| f.slug.clone()).collect();
        assert_eq!(slugs, vec!["a", "b", "c"]);
        assert_eq!(merged[1].name, "B new"); // unpinned "b" replaced by incoming
    }
}
