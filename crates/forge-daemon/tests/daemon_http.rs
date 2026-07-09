//! End-to-end daemon tests: real axum server on an ephemeral port, temp repo,
//! real TimelineStore + FeatureIndex.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use forge_core::{Feature, FeatureFile, FileRole};
use forge_daemon::{Daemon, DaemonDeps};
use forge_index::FeatureIndex;
use forge_timeline::TimelineStore;
use tokio::sync::RwLock;

fn seed_features_json(repo: &Path) {
    let features = vec![Feature {
        slug: "auth-flow".into(),
        name: "Auth flow".into(),
        description: "Login and session handling.".into(),
        entry_points: vec![PathBuf::from("src/auth/mod.rs")],
        files: vec![
            FeatureFile {
                path: PathBuf::from("src/auth/mod.rs"),
                role: FileRole::Core,
                pinned: false,
            },
            FeatureFile {
                path: PathBuf::from("src/auth/session.rs"),
                role: FileRole::Support,
                pinned: false,
            },
        ],
        tags: vec!["security".into()],
        confidence: 0.9,
        pinned: false,
        updated_at: Utc::now(),
    }];
    let dir = repo.join(".featureforge");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("features.json"),
        serde_json::to_string_pretty(&features).unwrap(),
    )
    .unwrap();
}

async fn start_daemon(repo: &Path) -> forge_daemon::DaemonHandle {
    seed_features_json(repo);
    let index = FeatureIndex::load(repo).unwrap();
    let timeline = TimelineStore::open(repo).unwrap();
    let deps = DaemonDeps {
        repo_root: repo.to_path_buf(),
        index: Arc::new(RwLock::new(index)),
        timeline: Arc::new(timeline),
    };
    Daemon::start(repo, deps).await.unwrap()
}

#[tokio::test]
async fn hook_post_lands_classified_timeline_row() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    let handle = start_daemon(repo).await;
    let base = format!("http://127.0.0.1:{}", handle.port);
    let client = reqwest::Client::new();

    // daemon.json advertises the bound port.
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo.join(".featureforge/runtime/daemon.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["port"].as_u64().unwrap() as u16, handle.port);
    assert!(manifest["pid"].is_number());
    assert!(manifest["started_at"].is_string());

    // PostToolUse Edit hook (absolute path, as Claude Code sends it).
    let edited = repo.join("src/auth/mod.rs");
    let hook = serde_json::json!({
        "session_id": "sess-123",
        "hook_event_name": "PostToolUse",
        "tool_name": "Edit",
        "tool_input": { "file_path": edited, "old_string": "a", "new_string": "b" }
    });
    let resp = client.post(format!("{base}/hooks/event")).json(&hook).send().await.unwrap();
    assert_eq!(resp.status(), 200);

    // Unrecognized event still gets a fast 200.
    let resp = client
        .post(format!("{base}/hooks/event"))
        .json(&serde_json::json!({ "hook_event_name": "PreCompact" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let events: Vec<forge_core::TimelineEvent> = client
        .get(format!("{base}/api/timeline"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(events.len(), 1, "only the recognized hook is recorded");
    let event = &events[0];
    assert_eq!(event.kind, forge_core::EventKind::FileEdited);
    assert_eq!(event.actor, forge_core::Actor::Agent);
    assert_eq!(event.session_id.as_deref(), Some("sess-123"));
    assert_eq!(event.feature_slugs, vec!["auth-flow".to_string()]);
    assert_eq!(event.payload["tool"], "Edit");

    handle.shutdown().await;
    assert!(
        !repo.join(".featureforge/runtime/daemon.json").exists(),
        "daemon.json removed on shutdown"
    );
}

#[tokio::test]
async fn feature_and_health_and_note_routes() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    let handle = start_daemon(repo).await;
    let base = format!("http://127.0.0.1:{}", handle.port);
    let client = reqwest::Client::new();

    let features: Vec<Feature> = client
        .get(format!("{base}/api/features"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(features.len(), 1);
    assert_eq!(features[0].slug, "auth-flow");

    let one: Feature = client
        .get(format!("{base}/api/features/auth-flow"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(one.name, "Auth flow");

    let missing = client
        .get(format!("{base}/api/features/nope"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);

    let health: serde_json::Value = client
        .get(format!("{base}/api/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(health["status"], "ok");
    assert_eq!(health["port"].as_u64().unwrap() as u16, handle.port);
    assert_eq!(health["repo"], serde_json::json!(repo));
    assert!(health["uptime"].is_number());

    let classified: serde_json::Value = client
        .get(format!("{base}/api/classify"))
        .query(&[("path", "src/auth/session.rs")])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(classified["featureSlugs"], serde_json::json!(["auth-flow"]));

    let note: forge_core::TimelineEvent = client
        .post(format!("{base}/api/notes"))
        .json(&serde_json::json!({ "text": "picked JWT", "featureSlugs": ["auth-flow"] }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(note.kind, forge_core::EventKind::Note);
    assert_eq!(note.actor, forge_core::Actor::Agent);
    assert_eq!(note.payload["text"], "picked JWT");
    assert_eq!(note.feature_slugs, vec!["auth-flow".to_string()]);

    // Filtered timeline sees the note.
    let events: Vec<forge_core::TimelineEvent> = client
        .get(format!("{base}/api/timeline"))
        .query(&[("feature", "auth-flow"), ("limit", "10")])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(events.iter().any(|e| e.id == note.id));

    handle.shutdown().await;
}
