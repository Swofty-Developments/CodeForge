//! Parsing of Claude Code hook payloads (stdin JSON forwarded by
//! `.codeforge/hooks/forward.sh`) into timeline events.

use std::path::PathBuf;

use forge_core::EventKind;
use serde_json::Value;

/// Tools whose PostToolUse hook means "a file was edited".
const EDIT_TOOLS: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

/// A hook payload mapped to a timeline event (before classification).
#[derive(Debug)]
pub(crate) struct ParsedHook {
    pub session_id: Option<String>,
    pub kind: EventKind,
    pub payload: Value,
    /// Paths to classify into feature slugs (empty for non-file events).
    pub edited_paths: Vec<PathBuf>,
}

/// Map a raw hook payload to a timeline event. `None` = unrecognized event
/// (callers log at debug and still reply 200).
pub(crate) fn parse_hook_event(raw: &Value) -> Option<ParsedHook> {
    let session_id = raw
        .get("session_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let event_name = raw.get("hook_event_name").and_then(Value::as_str)?;

    let (kind, payload, edited_paths) = match event_name {
        "PostToolUse" => parse_post_tool_use(raw)?,
        "Stop" => (EventKind::SessionEnded, serde_json::json!({}), vec![]),
        "SessionStart" => {
            let mut payload = serde_json::Map::new();
            if let Some(source) = raw.get("source").and_then(Value::as_str) {
                payload.insert("source".into(), source.into());
            }
            (EventKind::SessionStarted, Value::Object(payload), vec![])
        }
        _ => return None,
    };

    Some(ParsedHook {
        session_id,
        kind,
        payload,
        edited_paths,
    })
}

fn parse_post_tool_use(raw: &Value) -> Option<(EventKind, Value, Vec<PathBuf>)> {
    let tool_name = raw.get("tool_name").and_then(Value::as_str)?;
    let tool_input = raw.get("tool_input");
    let input_str = |key: &str| {
        tool_input
            .and_then(|i| i.get(key))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };

    if EDIT_TOOLS.contains(&tool_name) {
        let path = input_str("file_path").or_else(|| input_str("notebook_path"))?;
        let edited = vec![PathBuf::from(&path)];
        let payload = serde_json::json!({ "tool": tool_name, "path": path });
        Some((EventKind::FileEdited, payload, edited))
    } else if tool_name == "Bash" {
        let command = input_str("command")?;
        let kind = if is_test_command(&command) {
            EventKind::TestsRun
        } else {
            EventKind::CommandRun
        };
        let payload = serde_json::json!({
            "command": command,
            "description": input_str("description"),
        });
        Some((kind, payload, vec![]))
    } else {
        None
    }
}

/// Test-runner detection: standalone runners or `<tool> test` invocations,
/// anywhere in a compound command line.
fn is_test_command(command: &str) -> bool {
    const STANDALONE: [&str; 8] = [
        "pytest", "jest", "vitest", "mocha", "rspec", "phpunit", "tox", "ctest",
    ];
    const TEST_SUBCOMMAND: [&str; 9] = [
        "cargo", "go", "npm", "pnpm", "yarn", "bun", "dotnet", "mix", "mvn",
    ];
    let tokens: Vec<&str> = command
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '(' | ')'))
        .filter(|t| !t.is_empty() && *t != "&&" && *t != "||")
        .collect();
    tokens.iter().enumerate().any(|(i, tok)| {
        let base = tok.rsplit('/').next().unwrap_or(tok);
        if STANDALONE.contains(&base) {
            return true;
        }
        if base == "gradle" || base == "gradlew" {
            return tokens.get(i + 1).is_some_and(|next| *next == "test");
        }
        TEST_SUBCOMMAND.contains(&base)
            && tokens.get(i + 1).is_some_and(|next| {
                *next == "test"
                    || *next == "nextest"
                    || (*next == "run" && tokens.get(i + 2).is_some_and(|n| n.starts_with("test")))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_tool_use_edit_maps_to_file_edited() {
        let raw = serde_json::json!({
            "session_id": "s-1",
            "hook_event_name": "PostToolUse",
            "tool_name": "Edit",
            "tool_input": { "file_path": "/repo/src/main.rs", "old_string": "a" }
        });
        let parsed = parse_hook_event(&raw).unwrap();
        assert_eq!(parsed.kind, EventKind::FileEdited);
        assert_eq!(parsed.session_id.as_deref(), Some("s-1"));
        assert_eq!(parsed.payload["path"], "/repo/src/main.rs");
        assert_eq!(parsed.payload["tool"], "Edit");
        assert_eq!(parsed.edited_paths, vec![PathBuf::from("/repo/src/main.rs")]);
    }

    #[test]
    fn notebook_edit_uses_notebook_path() {
        let raw = serde_json::json!({
            "hook_event_name": "PostToolUse",
            "tool_name": "NotebookEdit",
            "tool_input": { "notebook_path": "/repo/nb.ipynb" }
        });
        let parsed = parse_hook_event(&raw).unwrap();
        assert_eq!(parsed.kind, EventKind::FileEdited);
        assert_eq!(parsed.payload["path"], "/repo/nb.ipynb");
    }

    #[test]
    fn bash_maps_to_command_run() {
        let raw = serde_json::json!({
            "hook_event_name": "PostToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": "cargo build", "description": "Build" }
        });
        let parsed = parse_hook_event(&raw).unwrap();
        assert_eq!(parsed.kind, EventKind::CommandRun);
        assert_eq!(parsed.payload["command"], "cargo build");
        assert_eq!(parsed.payload["description"], "Build");
        assert!(parsed.edited_paths.is_empty());
    }

    #[test]
    fn bash_test_commands_map_to_tests_run() {
        for cmd in [
            "cargo test",
            "cargo nextest run",
            "cd api && cargo test -p forge-core",
            "npm test",
            "npm run test:unit",
            "pnpm test --filter web",
            "go test ./...",
            "python -m pytest tests/",
            "./node_modules/.bin/vitest run",
            "./gradlew test",
        ] {
            let raw = serde_json::json!({
                "hook_event_name": "PostToolUse",
                "tool_name": "Bash",
                "tool_input": { "command": cmd }
            });
            let parsed = parse_hook_event(&raw).unwrap();
            assert_eq!(parsed.kind, EventKind::TestsRun, "command: {cmd}");
            assert_eq!(parsed.payload["command"], cmd);
        }
    }

    #[test]
    fn bash_non_test_commands_stay_command_run() {
        for cmd in [
            "cargo build",
            "npm run dev",
            "go build ./...",
            "ls tests/",
            "git commit -m 'test: add coverage'",
        ] {
            let raw = serde_json::json!({
                "hook_event_name": "PostToolUse",
                "tool_name": "Bash",
                "tool_input": { "command": cmd }
            });
            let parsed = parse_hook_event(&raw).unwrap();
            assert_eq!(parsed.kind, EventKind::CommandRun, "command: {cmd}");
        }
    }

    #[test]
    fn stop_and_session_start_map_to_session_events() {
        let stop = serde_json::json!({ "hook_event_name": "Stop", "session_id": "s" });
        assert_eq!(parse_hook_event(&stop).unwrap().kind, EventKind::SessionEnded);
        let start = serde_json::json!({ "hook_event_name": "SessionStart", "source": "startup" });
        let parsed = parse_hook_event(&start).unwrap();
        assert_eq!(parsed.kind, EventKind::SessionStarted);
        assert_eq!(parsed.payload["source"], "startup");
    }

    #[test]
    fn unrecognized_events_yield_none() {
        assert!(parse_hook_event(&serde_json::json!({ "hook_event_name": "PreToolUse" })).is_none());
        assert!(parse_hook_event(&serde_json::json!({
            "hook_event_name": "PostToolUse", "tool_name": "Read",
            "tool_input": { "file_path": "/x" }
        }))
        .is_none());
        assert!(parse_hook_event(&serde_json::json!({ "foo": 1 })).is_none());
    }
}
