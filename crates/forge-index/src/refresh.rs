//! Living-doc auto-refresh — the flywheel's write-back half: after an agent
//! turn touches a feature, fold what just happened (file edits, commands, and
//! the notes the agent recorded) into `.codeforge/docs/<slug>.md`.

use std::collections::HashMap;
use std::path::Path;

use forge_core::{EventKind, Feature, TimelineEvent};

use crate::headless::run_headless_claude;
use crate::indexer::{clamp_lines, DOCS_DIR};
use crate::prompt::{role_label, DOC_MAX_LINES};
use crate::{parse, Result};

/// Commands longer than this are truncated in the prompt rendering.
const MAX_COMMAND_CHARS: usize = 160;

/// Refresh (or first-write) one feature's living doc from a just-finished
/// agent turn. `recent_events` is that turn's session slice (file edits,
/// commands, notes). The existing doc is handed to the model verbatim; a
/// missing doc is the named fresh-doc state — the prompt asks for a first
/// version instead. The updated markdown is written atomically (tmp+rename).
/// Every failure is a real `Err`, never a warn-and-continue.
pub async fn refresh_feature_doc(
    repo_root: &Path,
    feature: &Feature,
    recent_events: &[TimelineEvent],
) -> Result<()> {
    let docs_dir = repo_root.join(DOCS_DIR);
    let doc_path = docs_dir.join(format!("{}.md", feature.slug));
    let existing = match tokio::fs::read_to_string(&doc_path).await {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None, // fresh-doc state
        Err(e) => return Err(e.into()),
    };

    let prompt = refresh_prompt(feature, existing.as_deref(), recent_events);
    let reply = run_headless_claude(repo_root, &prompt).await?;
    let body = clamp_lines(parse::strip_fences(&reply), DOC_MAX_LINES);

    tokio::fs::create_dir_all(&docs_dir).await?;
    let tmp = doc_path.with_extension("md.tmp");
    tokio::fs::write(&tmp, body.as_bytes()).await?;
    tokio::fs::rename(&tmp, &doc_path).await?;
    Ok(())
}

/// The REFRESH prompt: the feature, the existing doc verbatim (or the explicit
/// fresh-doc instruction), and a compact rendering of the turn's events.
fn refresh_prompt(
    feature: &Feature,
    existing_doc: Option<&str>,
    recent_events: &[TimelineEvent],
) -> String {
    let entries = bullet_list(feature.entry_points.iter().map(|p| format!("- {}", p.display())));
    let files = bullet_list(
        feature
            .files
            .iter()
            .map(|f| format!("- {} ({})", f.path.display(), role_label(f.role))),
    );
    let doc_block = match existing_doc {
        Some(doc) => format!("The EXISTING living doc, verbatim:\n---\n{}\n---", doc.trim_end()),
        None => "This feature has NO living doc yet — write the first version, grounded in the \
                 real code."
            .to_string(),
    };

    format!(
        r#"You maintain the living design doc for ONE feature of this repository. An agent
coding turn just finished and touched this feature; bring the doc up to date.

Feature: {name} ({slug})
Description: {description}
Entry points:
{entries}
Key files:
{files}

{doc_block}

What happened this turn:
{events}

Update the doc to reflect these changes:
- read the edited files (and any listed files you need) to verify against the real code
- revise sections the changes made stale
- fold in new invariants or gotchas learned this turn (notes are the decisions the
  agent recorded while working — capture their lessons)
- keep the structure: Purpose / How it works / Key files / Invariants & gotchas
- keep it under {max} lines

Output ONLY the Markdown document — no code fence around the whole thing, no
preamble, no closing remarks."#,
        name = feature.name,
        slug = feature.slug,
        description = feature.description,
        events = render_events(recent_events),
        max = DOC_MAX_LINES,
    )
}

fn bullet_list<I: Iterator<Item = String>>(items: I) -> String {
    let joined = items.collect::<Vec<_>>().join("\n");
    if joined.is_empty() { "(none listed)".to_string() } else { joined }
}

/// Compact rendering of a turn: edited files (deduped, with edit counts),
/// commands run (deduped, truncated), and notes verbatim. Other event kinds
/// carry nothing the doc pass can use and are not rendered.
fn render_events(events: &[TimelineEvent]) -> String {
    let mut edit_counts: HashMap<String, u32> = HashMap::new();
    let mut edit_order: Vec<String> = Vec::new();
    let mut commands: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    for event in events {
        match event.kind {
            EventKind::FileEdited => {
                if let Some(path) = str_field(event, "path") {
                    if !edit_counts.contains_key(&path) {
                        edit_order.push(path.clone());
                    }
                    *edit_counts.entry(path).or_insert(0) += 1;
                }
            }
            EventKind::CommandRun => {
                if let Some(cmd) = str_field(event, "command") {
                    let cmd = truncate_chars(&cmd, MAX_COMMAND_CHARS);
                    if !commands.contains(&cmd) {
                        commands.push(cmd);
                    }
                }
            }
            EventKind::Note => {
                if let Some(text) = str_field(event, "text") {
                    notes.push(text);
                }
            }
            _ => {}
        }
    }

    let edits = bullet_list(edit_order.into_iter().map(|path| {
        match edit_counts.get(&path).copied().unwrap_or(1) {
            0 | 1 => format!("- {path}"),
            n => format!("- {path} ({n} edits)"),
        }
    }));
    let commands = bullet_list(commands.into_iter().map(|c| format!("- {c}")));
    let notes = bullet_list(notes.into_iter().map(|n| format!("- {n}")));
    format!(
        "Files edited:\n{edits}\nCommands run:\n{commands}\nNotes the agent recorded \
         (decisions / lessons from the session):\n{notes}"
    )
}

fn str_field(event: &TimelineEvent, key: &str) -> Option<String> {
    event.payload.get(key).and_then(|v| v.as_str()).map(str::to_owned)
}

fn truncate_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((idx, _)) => format!("{}…", &text[..idx]),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use forge_core::{Actor, FeatureFile, FileRole};

    use super::*;

    fn feature() -> Feature {
        Feature {
            slug: "auth".into(),
            name: "Auth".into(),
            description: "Login flow.".into(),
            entry_points: vec![PathBuf::from("src/auth/mod.rs")],
            files: vec![FeatureFile {
                path: PathBuf::from("src/auth/session.rs"),
                role: FileRole::Core,
                pinned: false,
            }],
            tags: vec![],
            pinned: false,
            color: None,
            group: None,
            updated_at: Utc::now(),
        }
    }

    fn event(kind: EventKind, payload: serde_json::Value) -> TimelineEvent {
        TimelineEvent {
            id: 1,
            ts: Utc::now(),
            session_id: Some("s-1".into()),
            actor: Actor::Agent,
            kind,
            feature_slugs: vec!["auth".into()],
            payload,
        }
    }

    fn turn_events() -> Vec<TimelineEvent> {
        vec![
            event(EventKind::FileEdited, serde_json::json!({"tool": "Edit", "path": "src/auth/session.rs"})),
            event(EventKind::FileEdited, serde_json::json!({"tool": "Edit", "path": "src/auth/session.rs"})),
            event(EventKind::CommandRun, serde_json::json!({"command": "cargo test -p auth"})),
            event(EventKind::Note, serde_json::json!({"text": "tokens now rotate on refresh"})),
        ]
    }

    #[test]
    fn prompt_includes_existing_doc_verbatim_and_rendered_events() {
        let prompt = refresh_prompt(&feature(), Some("# Auth\n\nOld body.\n"), &turn_events());
        assert!(prompt.contains("Auth (auth)"));
        assert!(prompt.contains("# Auth\n\nOld body."), "existing doc verbatim");
        assert!(prompt.contains("- src/auth/session.rs (2 edits)"), "edits deduped with count");
        assert!(prompt.contains("- cargo test -p auth"));
        assert!(prompt.contains("- tokens now rotate on refresh"));
        assert!(prompt.contains("src/auth/mod.rs"), "entry points listed");
        assert!(!prompt.contains("NO living doc"));
    }

    #[test]
    fn missing_doc_is_the_named_fresh_doc_state() {
        let prompt = refresh_prompt(&feature(), None, &turn_events());
        assert!(prompt.contains("NO living doc yet"));
        assert!(prompt.contains("write the first version"));
    }

    #[test]
    fn render_events_dedupes_commands_and_names_empty_sections() {
        let events = vec![
            event(EventKind::CommandRun, serde_json::json!({"command": "ls"})),
            event(EventKind::CommandRun, serde_json::json!({"command": "ls"})),
        ];
        let rendered = render_events(&events);
        assert_eq!(rendered.matches("- ls").count(), 1, "repeated command rendered once");
        assert!(rendered.contains("Files edited:\n(none listed)"));
        assert!(rendered.contains("(decisions / lessons from the session):\n(none listed)"));
    }

    /// Live refresh against the real `claude` CLI on a tiny fixture repo.
    /// Run with `cargo test -p forge-index -- --ignored`.
    #[tokio::test]
    #[ignore = "spawns a real `claude` process"]
    async fn live_refresh_writes_a_doc() {
        let tmp = crate::testutil::TempDir::new("refresh-live");
        tmp.write("src/auth/mod.rs", "pub mod session;\n");
        tmp.write(
            "src/auth/session.rs",
            "pub fn login(user: &str) -> String { format!(\"token-{user}\") }\n",
        );
        let mut f = feature();
        f.files[0].path = PathBuf::from("src/auth/session.rs");

        refresh_feature_doc(tmp.path(), &f, &turn_events())
            .await
            .expect("refresh should succeed");
        let doc = std::fs::read_to_string(tmp.path().join(".codeforge/docs/auth.md"))
            .expect("doc written");
        assert!(!doc.trim().is_empty());
    }
}
