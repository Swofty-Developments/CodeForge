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
        pinned: false,
        color: None,
        group: None,
        updated_at: Utc::now(),
    }];
    let dir = repo.join(".codeforge");
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
        &std::fs::read_to_string(repo.join(".codeforge/runtime/daemon.json")).unwrap(),
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
        !repo.join(".codeforge/runtime/daemon.json").exists(),
        "daemon.json removed on shutdown"
    );
}

#[tokio::test]
async fn daemon_drains_spooled_payloads_on_start() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    seed_features_json(repo);

    // Simulate forward.sh having spooled a payload while the app was closed.
    let spool = repo.join(".codeforge/runtime/spool");
    std::fs::create_dir_all(&spool).unwrap();
    let edited = repo.join("src/auth/mod.rs");
    std::fs::write(
        spool.join("spool.aaaaaa"),
        serde_json::json!({
            "session_id": "spooled-1",
            "hook_event_name": "PostToolUse",
            "tool_name": "Edit",
            "tool_input": { "file_path": edited, "old_string": "a", "new_string": "b" }
        })
        .to_string(),
    )
    .unwrap();
    // A garbage payload must be quarantined, not silently dropped.
    std::fs::write(spool.join("spool.bbbbbb"), "not json at all").unwrap();

    // start_daemon (via Daemon::start) drains the spool before serving.
    let index = FeatureIndex::load(repo).unwrap();
    let timeline = TimelineStore::open(repo).unwrap();
    let deps = DaemonDeps {
        repo_root: repo.to_path_buf(),
        index: Arc::new(RwLock::new(index)),
        timeline: Arc::new(timeline),
    };
    let handle = Daemon::start(repo, deps).await.unwrap();
    let base = format!("http://127.0.0.1:{}", handle.port);
    let client = reqwest::Client::new();

    let events: Vec<forge_core::TimelineEvent> = client
        .get(format!("{base}/api/timeline"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let drained = events
        .iter()
        .find(|e| e.session_id.as_deref() == Some("spooled-1"))
        .expect("spooled payload replayed onto the timeline");
    assert_eq!(drained.kind, forge_core::EventKind::FileEdited);
    assert_eq!(drained.feature_slugs, vec!["auth-flow".to_string()]);

    // Valid file removed; garbage quarantined under rejected/.
    assert!(!spool.join("spool.aaaaaa").exists());
    assert!(spool.join("rejected/spool.bbbbbb").exists());

    handle.shutdown().await;
}

#[tokio::test]
async fn unclassified_file_edit_marks_event_and_enqueues() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    let handle = start_daemon(repo).await; // seeds only the src/auth/* feature
    let base = format!("http://127.0.0.1:{}", handle.port);
    let client = reqwest::Client::new();

    // Edit a path that shares no directory with any indexed feature → the daemon
    // cannot classify it. This is the "stale index / new file" state, distinct
    // from a non-file event.
    let edited = repo.join("notes/scratch.md");
    let hook = serde_json::json!({
        "session_id": "sess-x",
        "hook_event_name": "PostToolUse",
        "tool_name": "Write",
        "tool_input": { "file_path": edited, "content": "hello" }
    });
    let resp = client.post(format!("{base}/hooks/event")).json(&hook).send().await.unwrap();
    assert_eq!(resp.status(), 200);

    // The event is stored with empty slugs but EXPLICITLY marked unclassified,
    // so the timeline UI can show "edit to an unindexed file — re-index pending".
    let events: Vec<forge_core::TimelineEvent> = client
        .get(format!("{base}/api/timeline"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let edit = events
        .iter()
        .find(|e| e.kind == forge_core::EventKind::FileEdited)
        .expect("file edit recorded");
    assert!(edit.feature_slugs.is_empty());
    assert_eq!(edit.payload["unclassified"], serde_json::json!(true));

    // …and the path is queued on the server-side reindex queue for a later
    // reindex to resolve, rather than losing the file→feature link forever.
    let queue = forge_daemon::ReindexQueue::open(repo).unwrap();
    let pending = queue.pending().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/scratch.md");
    assert_eq!(pending[0].event_id, edit.id);
    assert_eq!(pending[0].session_id.as_deref(), Some("sess-x"));

    handle.shutdown().await;
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
