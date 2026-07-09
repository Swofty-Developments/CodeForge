//! Groups [`FileDiff`]s by feature via a path→slugs classifier.

use std::collections::BTreeMap;

use forge_core::{DiffByFeature, FeatureDiffGroup, FileDiff};

pub(crate) const UNMAPPED_SLUG: &str = "unmapped";
const UNMAPPED_NAME: &str = "Unmapped";

/// `classify` maps a repo-relative path to the slugs of the features it belongs
/// to (empty = unmapped); `name_of` resolves a slug to its display name.
/// Groups are ordered by total changed lines desc, the unmapped bucket last.
pub(crate) fn group_by_feature(
    files: Vec<FileDiff>,
    classify: impl Fn(&str) -> Vec<String>,
    name_of: impl Fn(&str) -> Option<String>,
) -> DiffByFeature {
    // slug → (files, contains a file shared with another feature)
    let mut buckets: BTreeMap<String, (Vec<FileDiff>, bool)> = BTreeMap::new();
    for file in files {
        let slugs = classify(&file.path);
        if slugs.is_empty() {
            buckets.entry(UNMAPPED_SLUG.to_string()).or_default().0.push(file);
            continue;
        }
        let shared = slugs.len() > 1;
        for slug in &slugs {
            let bucket = buckets.entry(slug.clone()).or_default();
            bucket.0.push(file.clone());
            bucket.1 |= shared;
        }
    }

    let mut groups: Vec<FeatureDiffGroup> = buckets
        .into_iter()
        .map(|(slug, (files, shared))| {
            let unmapped = slug == UNMAPPED_SLUG;
            let name = if unmapped {
                UNMAPPED_NAME.to_string()
            } else {
                name_of(&slug).unwrap_or_else(|| slug.clone())
            };
            FeatureDiffGroup { slug, name, shared, unmapped, files }
        })
        .collect();
    groups.sort_by(|a, b| {
        // Order by the typed flag, not by sniffing the slug string.
        a.unmapped
            .cmp(&b.unmapped)
            .then_with(|| changed_lines(b).cmp(&changed_lines(a)))
            .then_with(|| a.slug.cmp(&b.slug))
    });
    DiffByFeature { groups }
}

fn changed_lines(group: &FeatureDiffGroup) -> u32 {
    group.files.iter().map(|f| f.additions + f.deletions).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fd(path: &str, additions: u32, deletions: u32) -> FileDiff {
        FileDiff {
            path: path.into(),
            status: "modified".into(),
            hunks: Vec::new(),
            additions,
            deletions,
            binary: false,
            truncated: false,
        }
    }

    fn classify(path: &str) -> Vec<String> {
        match path {
            "shared.rs" => vec!["auth".into(), "billing".into()],
            "auth.rs" => vec!["auth".into()],
            "invoice.rs" => vec!["billing".into()],
            _ => vec![],
        }
    }

    fn name_of(slug: &str) -> Option<String> {
        match slug {
            "auth" => Some("Auth".into()),
            _ => None,
        }
    }

    #[test]
    fn shared_file_appears_in_each_group() {
        let d = group_by_feature(vec![fd("shared.rs", 5, 0), fd("auth.rs", 1, 0)], classify, name_of);
        assert_eq!(d.groups.len(), 2);
        let auth = d.groups.iter().find(|g| g.slug == "auth").unwrap();
        let billing = d.groups.iter().find(|g| g.slug == "billing").unwrap();
        assert!(auth.files.iter().any(|f| f.path == "shared.rs"));
        assert!(billing.files.iter().any(|f| f.path == "shared.rs"));
        assert!(auth.shared);
        assert!(billing.shared);
        assert_eq!(auth.name, "Auth");
        // no display name known → slug fallback
        assert_eq!(billing.name, "billing");
    }

    #[test]
    fn unmapped_bucket_is_synthesized_and_last() {
        let d = group_by_feature(
            vec![fd("auth.rs", 1, 0), fd("mystery.txt", 100, 100)],
            classify,
            name_of,
        );
        assert_eq!(d.groups.len(), 2);
        let last = d.groups.last().unwrap();
        assert_eq!(last.slug, "unmapped");
        assert_eq!(last.name, "Unmapped");
        assert!(last.unmapped, "synthetic bucket carries the typed unmapped flag");
        assert!(!last.shared);
        assert_eq!(last.files[0].path, "mystery.txt");
        // unmapped stays last even with the most changed lines
        assert_eq!(d.groups[0].slug, "auth");
        assert!(!d.groups[0].unmapped);
    }

    #[test]
    fn groups_ordered_by_changed_lines_desc() {
        let d = group_by_feature(
            vec![fd("auth.rs", 2, 1), fd("invoice.rs", 10, 5)],
            classify,
            name_of,
        );
        let slugs: Vec<_> = d.groups.iter().map(|g| g.slug.as_str()).collect();
        assert_eq!(slugs, vec!["billing", "auth"]);
    }

    #[test]
    fn single_feature_file_not_marked_shared() {
        let d = group_by_feature(vec![fd("auth.rs", 1, 0)], classify, name_of);
        assert_eq!(d.groups.len(), 1);
        assert!(!d.groups[0].shared);
    }

    #[test]
    fn empty_input_yields_empty_diff() {
        let d = group_by_feature(Vec::new(), classify, name_of);
        assert!(d.groups.is_empty());
    }
}
