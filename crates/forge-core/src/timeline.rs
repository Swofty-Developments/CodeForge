use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One immutable entry in the per-repo append-only timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    /// SQLite rowid; append-only, never mutated.
    pub id: i64,
    /// Locked at write time.
    pub ts: DateTime<Utc>,
    /// Claude session uuid if agent-originated.
    pub session_id: Option<String>,
    pub actor: Actor,
    pub kind: EventKind,
    /// Classification result (may be empty until classified).
    pub feature_slugs: Vec<String>,
    /// Kind-specific payload.
    pub payload: serde_json::Value,
}

/// Who caused a timeline event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Agent,
    Human,
    System,
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    SessionStarted,
    SessionEnded,
    FileEdited,
    CommandRun,
    TestsRun,
    IndexStarted,
    IndexCompleted,
    FeaturePinned,
    FeatureEdited,
    /// A feature's living doc was auto-refreshed after an agent turn touched it.
    /// Payload: `{ slug, outcome: "updated" | "failed", detail }`.
    DocUpdated,
    Note,
}

/// Query filter for `get_timeline` / `TimelineStore::query`. All fields optional (AND-ed).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<Actor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<EventKind>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<DateTime<Utc>>,
    /// Backward-paging cursor: only events with `id` strictly below this
    /// (rowids are append-only, so id order == time order).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_id: Option<i64>,
    /// Max events returned, newest first. `None` = store default (200).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_roundtrip_snake_case_enums() {
        let e = TimelineEvent {
            id: 42,
            ts: Utc::now(),
            session_id: Some("abc".into()),
            actor: Actor::Agent,
            kind: EventKind::FileEdited,
            feature_slugs: vec!["auth-flow".into()],
            payload: serde_json::json!({ "path": "src/auth/mod.rs" }),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"actor\":\"agent\""));
        assert!(json.contains("\"kind\":\"file_edited\""));
        assert!(json.contains("\"sessionId\""));
        assert!(json.contains("\"featureSlugs\""));
        let back: TimelineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn filter_defaults_and_roundtrip() {
        let f: TimelineFilter = serde_json::from_str("{}").unwrap();
        assert_eq!(f, TimelineFilter::default());
        let f = TimelineFilter {
            feature_slug: Some("x".into()),
            actor: Some(Actor::Human),
            kinds: Some(vec![EventKind::Note, EventKind::TestsRun]),
            since: None,
            before_id: Some(400),
            limit: Some(50),
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains("\"featureSlug\""));
        assert!(json.contains("\"tests_run\""));
        let back: TimelineFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }
}
