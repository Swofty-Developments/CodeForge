//! Integration test: actually spawns Claude Code, sends a message, and verifies
//! the full streaming event lifecycle (TurnStarted → ContentDelta(s) → TurnCompleted).
//!
//! Requires `claude` to be installed and authenticated on the system.
//! Run with: cargo test -p codeforge-session --test claude_integration -- --nocapture

use std::time::Duration;

use codeforge_session::claude::ClaudeSession;
use codeforge_session::AgentEvent;
use tokio::time::timeout;

/// Send a single message to Claude Code and verify we get streaming events back.
#[tokio::test]
async fn claude_code_send_and_receive() {
    let cwd = std::env::temp_dir();

    let (session, mut rx) = ClaudeSession::start(&cwd, None)
        .await
        .expect("Failed to start Claude session — is `claude` installed?");

    // Send a message immediately — with -p mode, the init event comes after
    // the first message is sent, not before.
    session
        .send_message("Reply with exactly the word 'pong' and nothing else.")
        .expect("Failed to send message");

    // Collect events until TurnCompleted or timeout
    let mut got_session_ready = false;
    let mut got_turn_started = false;
    let mut got_content = false;
    let mut content_delta_count: usize = 0;
    let mut accumulated_text = String::new();
    let mut got_turn_completed = false;
    let mut got_usage = false;

    let response_timeout = Duration::from_secs(90);
    let deadline = tokio::time::Instant::now() + response_timeout;

    while tokio::time::Instant::now() < deadline {
        match timeout(Duration::from_secs(30), rx.recv()).await {
            Ok(Some(event)) => {
                match &event {
                    AgentEvent::SessionReady => {
                        println!("[OK] SessionReady");
                        got_session_ready = true;
                    }
                    AgentEvent::TurnStarted { turn_id } => {
                        println!("[OK] TurnStarted: {turn_id}");
                        got_turn_started = true;
                    }
                    AgentEvent::ContentDelta { text } => {
                        print!("{text}");
                        accumulated_text.push_str(text);
                        content_delta_count += 1;
                        got_content = true;
                    }
                    AgentEvent::TurnCompleted { turn_id } => {
                        println!("\n[OK] TurnCompleted: {turn_id}");
                        got_turn_completed = true;
                        break;
                    }
                    AgentEvent::UsageReport {
                        input_tokens,
                        output_tokens,
                        cost_usd,
                        model,
                        ..
                    } => {
                        println!(
                            "[OK] UsageReport: model={model}, in={input_tokens}, out={output_tokens}, cost=${cost_usd:.6}"
                        );
                        got_usage = true;
                    }
                    AgentEvent::SessionError { message } => {
                        panic!("SessionError: {message}");
                    }
                    other => {
                        println!("[..] Event: {:?}", other);
                    }
                }
            }
            Ok(None) => {
                println!("[!!] Channel closed");
                break;
            }
            Err(_) => {
                println!("[!!] Timeout waiting for event");
                break;
            }
        }
    }

    // Assertions
    assert!(got_session_ready, "Never received SessionReady");
    assert!(got_turn_started, "Never received TurnStarted");
    assert!(got_content, "Never received any ContentDelta");
    assert!(
        !accumulated_text.is_empty(),
        "Accumulated response text is empty"
    );
    assert!(got_turn_completed, "Never received TurnCompleted");

    println!("\n--- Full response ---");
    println!("{accumulated_text}");
    println!("--- End ---");
    println!(
        "Response length: {} chars, delta count: {}, got usage report: {}",
        accumulated_text.len(),
        content_delta_count,
        got_usage
    );

    // The response should contain "pong" since we asked for it
    let lower = accumulated_text.to_lowercase();
    assert!(
        lower.contains("pong"),
        "Expected response to contain 'pong', got: {accumulated_text}"
    );

    // No duplicate content: the response for "pong" should be very short.
    // If stream_event and assistant both emit, we'd see ~2x the expected length.
    assert!(
        accumulated_text.len() < 50,
        "Response suspiciously long — possible duplicate streaming. Got {} chars: {accumulated_text}",
        accumulated_text.len()
    );
}

/// Test multi-turn conversation: send two messages, verify context is preserved.
#[tokio::test]
async fn claude_code_multi_turn() {
    let cwd = std::env::temp_dir();

    let (session, mut rx) = ClaudeSession::start(&cwd, None)
        .await
        .expect("Failed to start Claude session");

    // First turn: send message immediately
    session
        .send_message("Remember the number 42. Reply only with 'noted'.")
        .expect("Failed to send first message");

    let response1 = collect_response(&mut rx).await;
    println!("[Turn 1] {response1}");
    assert!(
        response1.to_lowercase().contains("noted"),
        "Expected 'noted' in first response, got: {response1}"
    );

    // Second turn: verify context is preserved
    session
        .send_message("What number did I ask you to remember? Reply with just the number.")
        .expect("Failed to send second message");

    let response2 = collect_response(&mut rx).await;
    println!("[Turn 2] {response2}");
    assert!(
        response2.contains("42"),
        "Expected '42' in second response, got: {response2}"
    );
}

/// Verify that responses actually stream incrementally (multiple ContentDelta events),
/// not as a single dump.
#[tokio::test]
async fn claude_code_streams_incrementally() {
    let cwd = std::env::temp_dir();

    let (session, mut rx) = ClaudeSession::start(&cwd, None)
        .await
        .expect("Failed to start Claude session");

    // Ask for a longer response to ensure multiple streaming chunks
    session
        .send_message("Count from 1 to 10, one number per line.")
        .expect("Failed to send message");

    let mut delta_count: usize = 0;
    let mut text = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);

    while tokio::time::Instant::now() < deadline {
        match timeout(Duration::from_secs(30), rx.recv()).await {
            Ok(Some(AgentEvent::ContentDelta { text: t })) => {
                delta_count += 1;
                text.push_str(&t);
            }
            Ok(Some(AgentEvent::TurnCompleted { .. })) => break,
            Ok(Some(AgentEvent::SessionError { message })) => panic!("SessionError: {message}"),
            Ok(Some(_)) => continue,
            Ok(None) | Err(_) => break,
        }
    }

    println!("Got {delta_count} content deltas for response:\n{text}");

    // A counting response should produce multiple streaming chunks
    assert!(
        delta_count > 1,
        "Expected multiple ContentDelta events for incremental streaming, got {delta_count}. \
         Response may be arriving as a single dump instead of streaming."
    );

    // Verify the content is reasonable
    assert!(text.contains("1"), "Response should contain '1'");
    assert!(text.contains("10"), "Response should contain '10'");
}

// ── Helpers ──

async fn collect_response(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<AgentEvent>,
) -> String {
    let mut text = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);

    while tokio::time::Instant::now() < deadline {
        match timeout(Duration::from_secs(30), rx.recv()).await {
            Ok(Some(AgentEvent::ContentDelta { text: t })) => text.push_str(&t),
            Ok(Some(AgentEvent::TurnCompleted { .. })) => break,
            Ok(Some(AgentEvent::SessionError { message })) => panic!("SessionError: {message}"),
            Ok(Some(_)) => continue,
            Ok(None) => break,
            Err(_) => break,
        }
    }

    text
}
