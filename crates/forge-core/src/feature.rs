use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A feature of the indexed repository — the primary unit FeatureForge organises code by.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    /// Kebab-case id, stable across re-indexes.
    pub slug: String,
    pub name: String,
    /// 1-3 sentences.
    pub description: String,
    /// The "start reading here" files.
    pub entry_points: Vec<PathBuf>,
    /// Many-to-many: files participate in multiple features.
    pub files: Vec<FeatureFile>,
    pub tags: Vec<String>,
    /// Indexer confidence 0..1.
    pub confidence: f32,
    /// Human-pinned features survive re-index verbatim.
    pub pinned: bool,
    pub updated_at: DateTime<Utc>,
}

/// A file's membership in a feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFile {
    pub path: PathBuf,
    pub role: FileRole,
    pub pinned: bool,
}

/// The role a file plays within a feature. `Unknown` is a distinct, visible
/// state for a role the indexer did not classify — never conflated with `Support`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileRole {
    Core,
    Support,
    Test,
    Config,
    Unknown,
}

/// Partial human edit to a feature (`update_feature` IPC command). `None` = leave unchanged.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_feature() -> Feature {
        Feature {
            slug: "auth-flow".into(),
            name: "Auth flow".into(),
            description: "Login and session handling.".into(),
            entry_points: vec![PathBuf::from("src/auth/mod.rs")],
            files: vec![FeatureFile {
                path: PathBuf::from("src/auth/session.rs"),
                role: FileRole::Core,
                pinned: false,
            }],
            tags: vec!["security".into()],
            confidence: 0.9,
            pinned: true,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn feature_roundtrip_camel_case() {
        let f = sample_feature();
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains("\"entryPoints\""));
        assert!(json.contains("\"updatedAt\""));
        assert!(json.contains("\"role\":\"core\""));
        let back: Feature = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn feature_patch_roundtrip() {
        let p = FeaturePatch {
            name: Some("New name".into()),
            description: None,
            tags: Some(vec!["a".into()]),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("description"));
        let back: FeaturePatch = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
        let empty: FeaturePatch = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, FeaturePatch::default());
    }
}
