//! Integration tests for `TimelineStore` against a real on-disk SQLite file.

use std::time::Duration;

use forge_core::{Actor, EventKind, TimelineFilter};
use forge_timeline::{NewEvent, TimelineStore};

fn open_temp() -> (tempfile::TempDir, TimelineStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = TimelineStore::open(dir.path()).expect("open store");
    (dir, store)
}

fn note(slugs: &[&str], text: &str) -> NewEvent {
    NewEvent {
        session_id: None,
        actor: Actor::Agent,
        kind: EventKind::Note,
        feature_slugs: slugs.iter().map(|s| s.to_string()).collect(),
        payload: serde_json::json!({ "text": text }),
    }
}

#[test]
fn append_query_roundtrip() {
    let (dir, store) = open_temp();
    assert!(store
        .db_path()
        .ends_with(".featureforge/runtime/timeline.db"));
    assert!(dir
        .path()
        .join(".featureforge/runtime/timeline.db")
        .exists());

    let stored = store
        .append(NewEvent {
            session_id: Some("sess-1".into()),
            actor: Actor::Human,
            kind: EventKind::FileEdited,
            feature_slugs: vec!["auth-flow".into(), "billing".into()],
            payload: serde_json::json!({ "path": "src/auth/mod.rs", "n": 3 }),
        })
        .expect("append");
    assert!(stored.id >= 1);

    let got = store.query(&TimelineFilter::default()).expect("query");
    assert_eq!(got, vec![stored]);
}

#[test]
fn filter_by_feature_slug_is_exact_containment() {
    let (_dir, store) = open_temp();
    let a = store.append(note(&["auth-flow", "billing"], "a")).expect("append");
    store.append(note(&["timeline"], "b")).expect("append");
    let c = store.append(note(&["auth-flow"], "c")).expect("append");
    store.append(note(&[], "d")).expect("append");

    let filter = TimelineFilter {
        feature_slug: Some("auth-flow".into()),
        ..Default::default()
    };
    let got = store.query(&filter).expect("query");
    assert_eq!(got, vec![c, a]); // newest first

    // Substring of a slug must not match.
    let filter = TimelineFilter {
        feature_slug: Some("auth".into()),
        ..Default::default()
    };
    assert!(store.query(&filter).expect("query").is_empty());
}

#[test]
fn filter_by_kinds_and_actor() {
    let (_dir, store) = open_temp();
    let e1 = store
        .append(NewEvent {
            kind: EventKind::TestsRun,
            ..note(&[], "tests")
        })
        .expect("append");
    store
        .append(NewEvent {
            kind: EventKind::CommandRun,
            ..note(&[], "cmd")
        })
        .expect("append");
    let e3 = store
        .append(NewEvent {
            actor: Actor::System,
            kind: EventKind::IndexCompleted,
            ..note(&[], "idx")
        })
        .expect("append");

    let filter = TimelineFilter {
        kinds: Some(vec![EventKind::TestsRun, EventKind::IndexCompleted]),
        ..Default::default()
    };
    assert_eq!(store.query(&filter).expect("query"), vec![e3.clone(), e1]);

    let filter = TimelineFilter {
        actor: Some(Actor::System),
        ..Default::default()
    };
    assert_eq!(store.query(&filter).expect("query"), vec![e3]);

    let filter = TimelineFilter {
        kinds: Some(vec![]),
        ..Default::default()
    };
    assert!(store.query(&filter).expect("query").is_empty());
}

#[test]
fn filter_since_cutoff() {
    let (_dir, store) = open_temp();
    store.append(note(&[], "old")).expect("append");
    std::thread::sleep(Duration::from_millis(5));
    let cutoff = chrono::Utc::now();
    std::thread::sleep(Duration::from_millis(5));
    let recent = store.append(note(&[], "new")).expect("append");

    let filter = TimelineFilter {
        since: Some(cutoff),
        ..Default::default()
    };
    assert_eq!(store.query(&filter).expect("query"), vec![recent]);
}

#[test]
fn limit_defaults_to_200_newest_first() {
    let (_dir, store) = open_temp();
    for i in 0..205 {
        store.append(note(&[], &format!("e{i}"))).expect("append");
    }

    let got = store.query(&TimelineFilter::default()).expect("query");
    assert_eq!(got.len(), 200);
    assert_eq!(got[0].payload["text"], "e204");
    assert_eq!(got[199].payload["text"], "e5");

    let filter = TimelineFilter {
        limit: Some(3),
        ..Default::default()
    };
    let got = store.query(&filter).expect("query");
    assert_eq!(got.len(), 3);
    assert_eq!(got[0].payload["text"], "e204");
}

#[test]
fn subscribe_receives_appended_event() {
    let (_dir, store) = open_temp();
    let mut rx = store.subscribe();
    let stored = store.append(note(&["live"], "hello")).expect("append");
    assert_eq!(rx.try_recv().expect("broadcast"), stored);
}

#[test]
fn reopen_persists_events() {
    let dir = tempfile::tempdir().expect("tempdir");
    let stored = {
        let store = TimelineStore::open(dir.path()).expect("open");
        store.append(note(&["persist"], "kept")).expect("append")
    };
    let store = TimelineStore::open(dir.path()).expect("reopen");
    assert_eq!(store.query(&TimelineFilter::default()).expect("query"), vec![stored]);
}

#[test]
fn concurrent_appends_all_land() {
    let (_dir, store) = open_temp();
    let handles: Vec<_> = (0..8)
        .map(|t| {
            let store = store.clone();
            std::thread::spawn(move || {
                for i in 0..25 {
                    store
                        .append(note(&["shared"], &format!("t{t}-{i}")))
                        .expect("append");
                }
            })
        })
        .collect();
    for h in handles {
        h.join().expect("thread panicked");
    }

    let filter = TimelineFilter {
        limit: Some(1000),
        ..Default::default()
    };
    let got = store.query(&filter).expect("query");
    assert_eq!(got.len(), 200);
    let mut ids: Vec<i64> = got.iter().map(|e| e.id).collect();
    ids.dedup();
    assert_eq!(ids.len(), 200);
    // Newest-first rowid ordering holds under concurrency.
    assert!(ids.windows(2).all(|w| w[0] > w[1]));
}
