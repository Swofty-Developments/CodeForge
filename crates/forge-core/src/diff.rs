use serde::{Deserialize, Serialize};

/// The full pending diff of a repo, grouped by feature (`get_diff_by_feature`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffByFeature {
    pub groups: Vec<FeatureDiffGroup>,
}

/// One accordion group in the diff review view. Files belonging to N features
/// appear under each of them with `shared: true`. Files matching no feature
/// fall under a synthetic "Unmapped" group (`slug: "unmapped"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDiffGroup {
    pub slug: String,
    pub name: String,
    pub shared: bool,
    pub files: Vec<FileDiff>,
}

/// A changed file with parsed hunks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    /// Repo-relative path.
    pub path: String,
    /// "modified" | "added" | "deleted" | "renamed" | "untracked".
    pub status: String,
    pub hunks: Vec<DiffHunk>,
    pub additions: u32,
    pub deletions: u32,
}

/// One `@@ … @@` hunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// One diff line. `origin` is '+' (add), '-' (remove) or ' ' (context) —
/// serialized as a 1-char JSON string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub origin: char,
    pub content: String,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_roundtrip_camel_case() {
        let d = DiffByFeature {
            groups: vec![FeatureDiffGroup {
                slug: "auth-flow".into(),
                name: "Auth flow".into(),
                shared: true,
                files: vec![FileDiff {
                    path: "src/auth/mod.rs".into(),
                    status: "modified".into(),
                    hunks: vec![DiffHunk {
                        header: "@@ -1,2 +1,3 @@".into(),
                        lines: vec![
                            DiffLine { origin: ' ', content: "fn main() {".into(), old_no: Some(1), new_no: Some(1) },
                            DiffLine { origin: '-', content: "old".into(), old_no: Some(2), new_no: None },
                            DiffLine { origin: '+', content: "new".into(), old_no: None, new_no: Some(2) },
                        ],
                    }],
                    additions: 1,
                    deletions: 1,
                }],
            }],
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("\"origin\":\"+\""));
        assert!(json.contains("\"oldNo\""));
        assert!(json.contains("\"newNo\""));
        let back: DiffByFeature = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn empty_diff_roundtrip() {
        let d = DiffByFeature::default();
        let back: DiffByFeature = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(d, back);
    }
}
