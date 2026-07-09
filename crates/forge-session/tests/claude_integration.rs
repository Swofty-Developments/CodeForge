//! Live integration tests: spawn the real Node agent-sidecar (which drives the
//! `claude` CLI via the Agent SDK) and assert the event lifecycle.
//!
//! `#[ignore]` — these make real model calls and require `node` plus an
//! authenticated `claude` install. Run with:
//!   `cargo test -p forge-session -- --ignored`

use std::time::Duration;

use forge_session::{AgentEvent, ClaudeSession};
use tokio::sync::mpsc;
use tokio::time::{timeout, Instant};

/// Max wait for any single event before giving up.
const EVENT_TIMEOUT: Duration = Duration::from_secs(30);
/// Overall budget per drain loop.
const OVERALL_DEADLINE: Duration = Duration::from_secs(90);

/// Drain events until `stop` matches (or the deadline / a closed channel /
/// an event-timeout ends it), returning everything received including the match.
async fn collect_until(
    rx: &mut mpsc::Receiver<AgentEvent>,
    mut stop: impl FnMut(&AgentEvent) -> bool,
) -> Vec<AgentEvent> {
    let deadline = Instant::now() + OVERALL_DEADLINE;
    let mut events = Vec::new();
    loop {
        let step = deadline.saturating_duration_since(Instant::now()).min(EVENT_TIMEOUT);
        if step.is_zero() {
            break;
        }
        match timeout(step, rx.recv()).await {
            Ok(Some(ev)) => {
                let done = stop(&ev);
                events.push(ev);
                if done {
                    break;
                }
            }
            Ok(None) | Err(_) => break, // channel closed or timed out
        }
    }
    events
}

fn is_boot_ready(e: &AgentEvent) -> bool {
    matches!(e, AgentEvent::SessionReady { claude_session_id: None, .. })
}

#[tokio::test]
#[ignore = "live: spawns real agent-sidecar + claude CLI"]
async fn lifecycle_ready_session_ready_text_turn_completed() {
    let cwd = tempfile::tempdir().expect("tempdir");
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(1024);

    let mut session = ClaudeSession::start(cwd.path(), Some("haiku"), None, tx)
        .await
        .expect("spawn sidecar");

    // Sidecar emits `ready` on boot → SessionReady with no session id yet.
    let boot = collect_until(&mut rx, is_boot_ready).await;
    assert!(
        matches!(boot.last(), Some(e) if is_boot_ready(e)),
        "expected initial ready event, got {boot:?}"
    );

    session.send_message("Reply with exactly the word: pong").expect("send");

    let events = collect_until(&mut rx, |e| matches!(e, AgentEvent::TurnCompleted { .. })).await;

    // Ordering: session_ready (with an id) → first text delta → turn_completed.
    let session_ready_idx = events
        .iter()
        .position(|e| matches!(e, AgentEvent::SessionReady { claude_session_id: Some(_), .. }))
        .expect("session_ready carrying an SDK session id");
    let first_text_idx = events
        .iter()
        .position(|e| matches!(e, AgentEvent::ContentDelta { .. }))
        .expect("at least one text delta");
    let turn_completed_idx = events
        .iter()
        .position(|e| matches!(e, AgentEvent::TurnCompleted { .. }))
        .expect("turn_completed");

    assert!(session_ready_idx < first_text_idx, "session_ready must precede text: {events:?}");
    assert!(first_text_idx < turn_completed_idx, "text must precede turn_completed: {events:?}");

    let text: String = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::ContentDelta { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(!text.trim().is_empty(), "expected non-empty streamed text, got {events:?}");

    // The SDK session id captured from session_ready is exposed on the handle.
    assert!(session.claude_session_id().is_some(), "session id captured from session_ready");

    session.stop().await.ok();
}

#[tokio::test]
#[ignore = "live: spawns real agent-sidecar + claude CLI"]
async fn abort_mid_turn_still_completes() {
    let cwd = tempfile::tempdir().expect("tempdir");
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(1024);

    let mut session = ClaudeSession::start(cwd.path(), Some("haiku"), None, tx)
        .await
        .expect("spawn sidecar");

    collect_until(&mut rx, is_boot_ready).await;

    session
        .send_message("Count slowly from 1 to 50, one number per line.")
        .expect("send");

    // Interrupt once the turn is genuinely under way.
    collect_until(&mut rx, |e| {
        matches!(
            e,
            AgentEvent::TurnStarted { .. }
                | AgentEvent::SessionReady { claude_session_id: Some(_), .. }
                | AgentEvent::ContentDelta { .. }
        )
    })
    .await;
    session.interrupt().expect("abort");

    // turn_completed must still fire (sidecar `finally` guarantee).
    let events = collect_until(&mut rx, |e| matches!(e, AgentEvent::TurnCompleted { .. })).await;
    assert!(
        events.iter().any(|e| matches!(e, AgentEvent::TurnCompleted { .. })),
        "turn_completed must fire even after abort: {events:?}"
    );

    session.stop().await.ok();
}
