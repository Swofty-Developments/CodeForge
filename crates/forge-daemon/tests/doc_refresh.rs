//! Doc-refresh worker tests over a real TimelineStore + FeatureIndex — no live
//! claude. The refresh failure is injected honestly: `deps.repo_root` points at
//! a directory that does not exist, so the headless pass fails fast (either the
//! `claude` PATH lookup or the spawn with a nonexistent cwd) before any model
//! call. Anything needing a live claude belongs in forge-index `#[ignore]` tests.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use forge_core::{Actor, EventKind, Feature, FeatureFile, FileRole, TimelineEvent, TimelineFilter};
use forge_daemon::{DaemonDeps, DocRefresher};
use forge_index::FeatureIndex;
use forge_timeline::{NewEvent, TimelineStore};
use tokio::sync::RwLock;

fn feature(slug: &str, pinned: bool) -> Feature {
    Feature {
        slug: slug.into(),
        name: slug.to_uppercase(),
        description: format!("The {slug} feature."),
        entry_points: vec![PathBuf::from(format!("src/{slug}.rs"))],
        files: vec![FeatureFile {
            path: PathBuf::from(format!("src/{slug}.rs")),
            role: FileRole::Core,
            pinned: false,
        }],
        tags: vec![],
        pinned,
        color: None,
        group: None,
        updated_at: Utc::now(),
    }
}

/// Deps whose index/timeline live in `dir` but whose `repo_root` is a
/// NONEXISTENT path — any doc refresh that actually runs fails fast.
fn deps_with_features(dir: &Path, features: Vec<Feature>) -> DaemonDeps {
    let mut index = FeatureIndex::load(dir).unwrap();
    for f in features {
        index.upsert(f);
    }
    DaemonDeps {
        repo_root: dir.join("missing-repo-root"),
        index: Arc::new(RwLock::new(index)),
        timeline: Arc::new(TimelineStore::open(dir).unwrap()),
    }
}

fn append(
    deps: &DaemonDeps,
    session: Option<&str>,
    kind: EventKind,
    slugs: &[&str],
    payload: serde_json::Value,
) -> i64 {
    deps.timeline
        .append(NewEvent {
            session_id: session.map(str::to_owned),
            actor: Actor::Agent,
            kind,
            feature_slugs: slugs.iter().map(|s| s.to_string()).collect(),
            payload,
        })
        .unwrap()
        .id
}

fn doc_updated_events(deps: &DaemonDeps) -> Vec<TimelineEvent> {
    deps.timeline
        .query(&TimelineFilter {
            kinds: Some(vec![EventKind::DocUpdated]),
            ..Default::default()
        })
        .unwrap()
}

#[tokio::test]
async fn pinned_feature_doc_is_never_auto_refreshed() {
    let tmp = tempfile::tempdir().unwrap();
    let deps = deps_with_features(tmp.path(), vec![feature("auth", true)]);

    append(&deps, Some("s1"), EventKind::SessionStarted, &[], serde_json::json!({}));
    append(
        &deps,
        Some("s1"),
        EventKind::FileEdited,
        &["auth"],
        serde_json::json!({"tool": "Edit", "path": "src/auth.rs"}),
    );
    let end = append(&deps, Some("s1"), EventKind::SessionEnded, &[], serde_json::json!({}));

    DocRefresher::new(deps.clone()).on_session_ended("s1", end).await;

    // Human-owned doc: no refresh ran, so no DocUpdated event of EITHER outcome.
    assert!(doc_updated_events(&deps).is_empty());
}

#[tokio::test]
async fn refresh_failure_lands_a_named_doc_updated_failed_event() {
    let tmp = tempfile::tempdir().unwrap();
    let deps = deps_with_features(tmp.path(), vec![feature("auth", false)]);

    append(&deps, Some("s1"), EventKind::SessionStarted, &[], serde_json::json!({}));
    append(
        &deps,
        Some("s1"),
        EventKind::FileEdited,
        &["auth"],
        serde_json::json!({"tool": "Edit", "path": "src/auth.rs"}),
    );
    let end = append(&deps, Some("s1"), EventKind::SessionEnded, &[], serde_json::json!({}));

    DocRefresher::new(deps.clone()).on_session_ended("s1", end).await;

    let events = doc_updated_events(&deps);
    assert_eq!(events.len(), 1, "one refresh attempted, one outcome recorded");
    let event = &events[0];
    assert_eq!(event.kind, EventKind::DocUpdated);
    assert_eq!(event.actor, Actor::System);
    assert_eq!(event.session_id.as_deref(), Some("s1"));
    assert_eq!(event.feature_slugs, vec!["auth".to_string()]);
    assert_eq!(event.payload["slug"], "auth");
    assert_eq!(event.payload["outcome"], "failed");
    let detail = event.payload["detail"].as_str().unwrap();
    assert!(!detail.is_empty(), "failure detail names the cause");
}

#[tokio::test]
async fn only_the_ending_sessions_touched_features_are_refreshed() {
    let tmp = tempfile::tempdir().unwrap();
    let deps =
        deps_with_features(tmp.path(), vec![feature("auth", false), feature("billing", false)]);

    // Two interleaved sessions; only s2 ends.
    append(&deps, Some("s1"), EventKind::SessionStarted, &[], serde_json::json!({}));
    append(
        &deps,
        Some("s1"),
        EventKind::FileEdited,
        &["auth"],
        serde_json::json!({"tool": "Edit", "path": "src/auth.rs"}),
    );
    append(&deps, Some("s2"), EventKind::SessionStarted, &[], serde_json::json!({}));
    append(
        &deps,
        Some("s2"),
        EventKind::FileEdited,
        &["billing"],
        serde_json::json!({"tool": "Edit", "path": "src/billing.rs"}),
    );
    let end = append(&deps, Some("s2"), EventKind::SessionEnded, &[], serde_json::json!({}));

    DocRefresher::new(deps.clone()).on_session_ended("s2", end).await;

    let events = doc_updated_events(&deps);
    assert_eq!(events.len(), 1, "only s2's touched feature was refreshed");
    assert_eq!(events[0].payload["slug"], "billing");
}
