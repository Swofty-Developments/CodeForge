//! Parsing + validation of the feature JSON returned by headless `claude`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge_core::{Feature, FeatureFile, FileRole};
use serde::Deserialize;

use crate::{Error, Result};

/// Feature shape as requested from the model (lenient superset of [`Feature`]).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawFeature {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entry_points: Vec<PathBuf>,
    #[serde(default)]
    pub files: Vec<RawFeatureFile>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub confidence: f32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawFeatureFile {
    pub path: PathBuf,
    #[serde(default)]
    pub role: Option<String>,
}

/// Parse the model's reply into raw features: strip markdown fences, then
/// serde-parse; if the reply wraps the array in prose, fall back to the
/// outermost `[...]` slice.
pub(crate) fn parse_features(text: &str) -> Result<Vec<RawFeature>> {
    let body = strip_fences(text);
    match serde_json::from_str(body) {
        Ok(features) => Ok(features),
        Err(first_err) => {
            let (start, end) = match (body.find('['), body.rfind(']')) {
                (Some(s), Some(e)) if s < e => (s, e),
                _ => return Err(Error::Json(first_err)),
            };
            serde_json::from_str(&body[start..=end]).map_err(Error::Json)
        }
    }
}

/// Strip a single outer markdown code fence (```json / ```markdown / bare ```)
/// if the whole text is wrapped in one; inner fences are preserved.
pub(crate) fn strip_fences(text: &str) -> &str {
    let trimmed = text.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    // Drop the info string ("json", "markdown", ...) up to the first newline.
    let Some((_, body)) = rest.split_once('\n') else {
        return trimmed;
    };
    match body.trim_end().strip_suffix("```") {
        Some(inner) => inner.trim(),
        None => trimmed,
    }
}

/// Validate raw features into domain [`Feature`]s: sanitize slugs, drop
/// duplicates, drop paths that don't resolve to files inside the repo, clamp
/// confidence to 0..1, and drop features left with no paths at all.
pub(crate) fn validate_features(repo_root: &Path, raw: Vec<RawFeature>) -> Result<Vec<Feature>> {
    let canonical_root = repo_root.canonicalize()?;
    let now = Utc::now();
    let mut seen: HashSet<String> = HashSet::new();
    let mut features: Vec<Feature> = Vec::with_capacity(raw.len());

    for feature in raw {
        let slug = sanitize_slug(&feature.slug);
        if slug.is_empty() || !seen.insert(slug.clone()) {
            tracing::warn!(raw_slug = %feature.slug, "dropping feature with empty or duplicate slug");
            continue;
        }
        let entry_points: Vec<PathBuf> = feature
            .entry_points
            .iter()
            .filter_map(|p| clamp_path(repo_root, &canonical_root, p))
            .collect();
        let files: Vec<FeatureFile> = feature
            .files
            .iter()
            .filter_map(|f| {
                clamp_path(repo_root, &canonical_root, &f.path).map(|path| FeatureFile {
                    path,
                    role: parse_role(f.role.as_deref()),
                    pinned: false,
                })
            })
            .collect();
        if entry_points.is_empty() && files.is_empty() {
            tracing::warn!(slug = %slug, "dropping feature with no existing files");
            continue;
        }
        features.push(Feature {
            slug,
            name: feature.name,
            description: feature.description,
            entry_points,
            files,
            tags: feature.tags,
            confidence: clamp_confidence(feature.confidence),
            pinned: false,
            updated_at: now,
        });
    }

    features.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(features)
}

/// Repo-relative path of an existing regular file inside the repo, else None.
/// Canonicalization rejects `../` escapes and dangling paths.
fn clamp_path(repo_root: &Path, canonical_root: &Path, path: &Path) -> Option<PathBuf> {
    let rel = if path.is_absolute() {
        path.strip_prefix(repo_root)
            .or_else(|_| path.strip_prefix(canonical_root))
            .ok()?
    } else {
        path
    };
    let canonical = repo_root.join(rel).canonicalize().ok()?;
    if !canonical.starts_with(canonical_root) || !canonical.is_file() {
        return None;
    }
    canonical.strip_prefix(canonical_root).ok().map(Path::to_path_buf)
}

fn sanitize_slug(raw: &str) -> String {
    let mut slug = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() {
            slug.extend(c.to_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn parse_role(role: Option<&str>) -> FileRole {
    match role.map(|r| r.trim().to_ascii_lowercase()).as_deref() {
        Some("core") => FileRole::Core,
        Some("support") => FileRole::Support,
        Some("test") | Some("tests") => FileRole::Test,
        Some("config") => FileRole::Config,
        other => {
            if let Some(unknown) = other {
                tracing::debug!(role = %unknown, "unknown file role, defaulting to support");
            }
            FileRole::Support
        }
    }
}

fn clamp_confidence(confidence: f32) -> f32 {
    if confidence.is_finite() {
        confidence.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn strips_json_fences() {
        assert_eq!(strip_fences("```json\n[1, 2]\n```"), "[1, 2]");
        assert_eq!(strip_fences("```markdown\n# Doc\n```rust\nfn x() {}\n```\n```"), "# Doc\n```rust\nfn x() {}\n```");
    }

    #[test]
    fn leaves_unfenced_text_alone() {
        assert_eq!(strip_fences("  [1]  "), "[1]");
        assert_eq!(strip_fences("```unterminated"), "```unterminated");
    }

    #[test]
    fn parses_bare_and_fenced_arrays() {
        let bare = r#"[{"slug":"a","name":"A","files":[{"path":"x.rs","role":"core"}]}]"#;
        assert_eq!(parse_features(bare).expect("bare").len(), 1);
        let fenced = format!("```json\n{bare}\n```");
        assert_eq!(parse_features(&fenced).expect("fenced").len(), 1);
    }

    #[test]
    fn parses_array_wrapped_in_prose() {
        let text = r#"Here are the features: [{"slug":"a","name":"A"}] hope that helps"#;
        assert_eq!(parse_features(text).expect("prose").len(), 1);
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(matches!(parse_features("not json at all"), Err(Error::Json(_))));
        assert!(matches!(parse_features(r#"{"slug":"not-an-array"}"#), Err(Error::Json(_))));
    }

    #[test]
    fn validate_drops_missing_files_and_clamps() {
        let tmp = TempDir::new("validate");
        tmp.write("src/a.rs", "fn a() {}");
        tmp.write("README.md", "# readme");

        let raw = parse_features(
            r#"[
              {"slug":"Real Feature!","name":"Real","entryPoints":["src/a.rs","ghost.rs"],
               "files":[{"path":"src/a.rs","role":"core"},{"path":"missing.rs","role":"test"},
                        {"path":"../escape.rs","role":"config"},{"path":"README.md","role":"whatever"}],
               "confidence":3.7},
              {"slug":"ghost-only","name":"Ghost","files":[{"path":"nope.rs","role":"core"}]},
              {"slug":"real-feature","name":"Duplicate slug","files":[{"path":"README.md","role":"core"}]}
            ]"#,
        )
        .expect("parse");
        let features = validate_features(tmp.path(), raw).expect("validate");

        assert_eq!(features.len(), 1);
        let f = &features[0];
        assert_eq!(f.slug, "real-feature");
        assert_eq!(f.entry_points, vec![PathBuf::from("src/a.rs")]);
        let paths: Vec<_> = f.files.iter().map(|x| x.path.clone()).collect();
        assert_eq!(paths, vec![PathBuf::from("src/a.rs"), PathBuf::from("README.md")]);
        assert_eq!(f.files[1].role, FileRole::Support); // unknown role defaults
        assert_eq!(f.confidence, 1.0);
        assert!(!f.pinned);
    }

    #[test]
    fn validate_accepts_absolute_paths_inside_repo() {
        let tmp = TempDir::new("abs");
        tmp.write("src/b.rs", "fn b() {}");
        let abs = tmp.path().join("src/b.rs").display().to_string();
        let raw = parse_features(&format!(
            r#"[{{"slug":"abs","name":"Abs","files":[{{"path":"{abs}","role":"core"}}]}}]"#
        ))
        .expect("parse");
        let features = validate_features(tmp.path(), raw).expect("validate");
        assert_eq!(features[0].files[0].path, PathBuf::from("src/b.rs"));
    }
}
