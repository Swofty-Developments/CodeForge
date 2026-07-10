//! Pure path→feature scoring used by [`crate::FeatureIndex::classify_paths`].
//!
//! Per feature, per path: entry-point match scores 4, exact file match 3
//! (summed across paths); otherwise the deepest shared directory prefix with
//! any of the feature's paths acts as a fallback match and tiebreak. Features
//! with any signal are returned best-first.

use std::path::{Component, Path, PathBuf};

use forge_core::Feature;

const ENTRY_POINT_SCORE: u32 = 4;
const FILE_SCORE: u32 = 3;

pub(crate) fn classify(repo_root: &Path, features: &[Feature], paths: &[PathBuf]) -> Vec<String> {
    let paths: Vec<Vec<String>> = paths
        .iter()
        .map(|p| components(repo_root, p))
        .filter(|c| !c.is_empty())
        .collect();
    if paths.is_empty() {
        return Vec::new();
    }

    // (exact-match score, deepest shared dir prefix, slug)
    let mut scored: Vec<(u32, usize, &str)> = Vec::new();
    for feature in features {
        let entry_points: Vec<Vec<String>> = feature
            .entry_points
            .iter()
            .map(|p| components(repo_root, p))
            .collect();
        let files: Vec<Vec<String>> = feature
            .files
            .iter()
            .map(|f| components(repo_root, &f.path))
            .collect();

        let mut exact = 0u32;
        let mut depth = 0usize;
        for path in &paths {
            if entry_points.iter().any(|e| e == path) {
                exact += ENTRY_POINT_SCORE;
            } else if files.iter().any(|f| f == path) {
                exact += FILE_SCORE;
            } else {
                let best = entry_points
                    .iter()
                    .chain(files.iter())
                    .map(|other| shared_dir_depth(path, other))
                    .max()
                    .unwrap_or(0);
                depth = depth.max(best);
            }
        }
        if exact > 0 || depth > 0 {
            scored.push((exact, depth, feature.slug.as_str()));
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(b.2)));
    let mut out: Vec<String> = Vec::with_capacity(scored.len());
    for (_, _, slug) in scored {
        if !out.iter().any(|s| s == slug) {
            out.push(slug.to_string());
        }
    }
    out
}

/// Normalised path components. Absolute paths are relativised against
/// `repo_root`; absolute paths outside the repo are unclassifiable (empty).
fn components(repo_root: &Path, path: &Path) -> Vec<String> {
    let rel = match path.strip_prefix(repo_root) {
        Ok(rel) => rel,
        Err(_) if path.is_absolute() => return Vec::new(),
        Err(_) => path,
    };
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

/// Leading directory components two file paths share (file names excluded);
/// repo-root files share no directory by definition.
fn shared_dir_depth(a: &[String], b: &[String]) -> usize {
    if a.len() < 2 || b.len() < 2 {
        return 0;
    }
    a[..a.len() - 1]
        .iter()
        .zip(b[..b.len() - 1].iter())
        .take_while(|(x, y)| x == y)
        .count()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use forge_core::{FeatureFile, FileRole};

    use super::*;

    const ROOT: &str = "/repo";

    fn feat(slug: &str, entries: &[&str], files: &[&str]) -> Feature {
        Feature {
            slug: slug.into(),
            name: slug.into(),
            description: String::new(),
            entry_points: entries.iter().map(PathBuf::from).collect(),
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

    fn run(features: &[Feature], paths: &[&str]) -> Vec<String> {
        let paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        classify(Path::new(ROOT), features, &paths)
    }

    #[test]
    fn ranks_entry_point_above_file_above_dir_prefix() {
        let features = vec![
            feat("entry", &["src/auth/mod.rs"], &[]),
            feat("file", &[], &["src/auth/mod.rs"]),
            feat("sibling", &[], &["src/auth/session.rs"]),
            feat("unrelated", &[], &["docs/readme.md"]),
        ];
        // entry(4) > file(3) > sibling(dir depth 2); unrelated shares no dir.
        assert_eq!(run(&features, &["src/auth/mod.rs"]), vec!["entry", "file", "sibling"]);
    }

    #[test]
    fn deeper_shared_directory_wins_the_tiebreak() {
        let features = vec![
            feat("deep", &[], &["src/a/b/c/x.rs"]),
            feat("shallow", &[], &["src/z.rs"]),
            feat("off", &[], &["other/w.rs"]),
        ];
        // No exact match; rank by shared-dir depth: deep(4) > shallow(1); off(0) excluded.
        assert_eq!(run(&features, &["src/a/b/c/y.rs"]), vec!["deep", "shallow"]);
    }

    #[test]
    fn features_with_no_signal_are_excluded() {
        let features = vec![feat("only", &[], &["src/a.rs"])];
        assert!(run(&features, &["totally/unrelated.rs"]).is_empty());
    }

    #[test]
    fn membership_is_deduped_and_scores_sum_across_paths() {
        let features = vec![
            feat("multi", &[], &["src/a.rs", "src/b.rs"]),
            feat("one", &[], &["src/a.rs"]),
        ];
        // "multi" matches both paths (3+3=6) and ranks above "one" (3); dedup → one entry each.
        assert_eq!(run(&features, &["src/a.rs", "src/b.rs"]), vec!["multi", "one"]);
    }

    #[test]
    fn absolute_repo_paths_are_relativised() {
        let features = vec![feat("abs", &["src/main.rs"], &[])];
        let abs = format!("{ROOT}/src/main.rs");
        assert_eq!(run(&features, &[&abs]), vec!["abs"]);
    }

    #[test]
    fn root_level_files_need_an_exact_match() {
        // Two root files share no directory, so only an exact hit classifies.
        let features = vec![feat("cfg", &[], &["Cargo.toml"])];
        assert!(run(&features, &["README.md"]).is_empty());
        assert_eq!(run(&features, &["Cargo.toml"]), vec!["cfg"]);
    }
}
