//! Integration-kit installer. Runs on every repo open; **idempotent** and never
//! clobbers user content:
//!
//! - `.claude/settings.json` — merge hooks: PostToolUse (Edit|Write|MultiEdit|
//!   NotebookEdit|Bash), Stop, SessionStart → `.featureforge/hooks/forward.sh`.
//! - `.mcp.json` — merge a `featureforge` stdio server → `~/.featureforge/bin/forge-mcp`.
//! - `CLAUDE.md` — append a marker-delimited section
//!   (`<!-- featureforge:start -->` … `<!-- featureforge:end -->`) telling agents
//!   to consult the feature index via MCP before exploring and to `record_note`
//!   decisions. Re-installs replace only the marked section.
//! - `.featureforge/` scaffold: `hooks/forward.sh` (0755) + `.gitignore` ignoring `runtime/`.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::Result;

/// Markers delimiting the FeatureForge-owned section of CLAUDE.md.
pub const CLAUDE_MD_START: &str = "<!-- featureforge:start -->";
pub const CLAUDE_MD_END: &str = "<!-- featureforge:end -->";

/// Hook command installed into `.claude/settings.json`. `$CLAUDE_PROJECT_DIR`
/// is expanded by Claude Code at hook time.
const HOOK_COMMAND: &str = "\"$CLAUDE_PROJECT_DIR\"/.featureforge/hooks/forward.sh";
/// Substring identifying our hook entries across re-installs.
const HOOK_MARKER: &str = ".featureforge/hooks/forward.sh";
const POST_TOOL_USE_MATCHER: &str = "Edit|Write|MultiEdit|NotebookEdit|Bash";

const FORWARD_SH: &str = r#"#!/bin/sh
# FeatureForge hook forwarder: pipes Claude Code hook JSON (stdin) to the local
# daemon. Always exits 0 so hooks never block Claude when the app is closed.
ROOT="${CLAUDE_PROJECT_DIR:-.}"
RUNTIME="$ROOT/.featureforge/runtime/daemon.json"
[ -f "$RUNTIME" ] || exit 0
PORT=$(sed -n 's/.*"port"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$RUNTIME" | head -n 1)
[ -n "$PORT" ] || exit 0
curl -s --max-time 2 -X POST -H "Content-Type: application/json" \
  --data-binary @- "http://127.0.0.1:$PORT/hooks/event" >/dev/null 2>&1 || true
exit 0
"#;

const CLAUDE_MD_BODY: &str = r#"## FeatureForge

This repository is indexed by FeatureForge. **Before exploring the codebase**, consult the
`featureforge` MCP tools — they are faster and more accurate than searching from scratch:

- `list_features` — every feature in this repo with a short description
- `get_feature(slug)` — one feature's description, entry points, and key files
- `which_features(path)` — which features a file path belongs to
- `feature_timeline(slug)` — recent changes to a feature, newest first

Record significant decisions and findings as you work via `record_note(text, feature_slugs)`
so they land on the repo timeline for future sessions."#;

/// What the installer did (all false = everything was already in place).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KitReport {
    pub hooks_installed: bool,
    pub mcp_registered: bool,
    pub claude_md_updated: bool,
    pub scaffold_created: bool,
    /// Files created or modified by this run.
    pub changed_files: Vec<PathBuf>,
}

/// Install (or repair) the integration kit into `repo_root`. `mcp_bin_path` is
/// where the `forge-mcp` binary was copied on app start
/// (`~/.featureforge/bin/forge-mcp`).
pub fn install_kit(repo_root: &Path, mcp_bin_path: &Path) -> Result<KitReport> {
    let mut report = KitReport::default();

    report.hooks_installed = install_hooks_settings(repo_root, &mut report)?;
    report.mcp_registered = install_mcp_json(repo_root, mcp_bin_path, &mut report)?;
    report.claude_md_updated = install_claude_md(repo_root, &mut report)?;
    report.scaffold_created = install_scaffold(repo_root, &mut report)?;

    Ok(report)
}

/// Write `content` only if it differs from what's on disk; records the change.
fn write_if_changed(path: &Path, content: &str, report: &mut KitReport) -> Result<bool> {
    if std::fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    report.changed_files.push(path.to_path_buf());
    Ok(true)
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>> {
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<Value>(&text)? {
            Value::Object(map) => Ok(map),
            other => Err(crate::Error::Other(format!(
                "{} is not a JSON object (found {other})",
                path.display()
            ))),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(e.into()),
    }
}

fn write_json(path: &Path, map: Map<String, Value>, report: &mut KitReport) -> Result<bool> {
    let text = format!("{:#}\n", Value::Object(map));
    write_if_changed(path, &text, report)
}

/// True if any hook group in `groups` already runs forward.sh.
fn has_our_hook(groups: &[Value]) -> bool {
    groups.iter().any(|group| {
        group
            .get("hooks")
            .and_then(Value::as_array)
            .is_some_and(|hooks| {
                hooks.iter().any(|h| {
                    h.get("command")
                        .and_then(Value::as_str)
                        .is_some_and(|c| c.contains(HOOK_MARKER))
                })
            })
    })
}

/// Merge our hook entries into `.claude/settings.json`, preserving every
/// pre-existing key and hook. Current Claude Code schema:
/// `hooks.<Event>: [{matcher?, hooks: [{type: "command", command}]}]`.
fn install_hooks_settings(repo_root: &Path, report: &mut KitReport) -> Result<bool> {
    let path = repo_root.join(".claude").join("settings.json");
    let mut settings = read_json_object(&path)?;

    let hooks = settings
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| crate::Error::Other("settings.json: \"hooks\" is not an object".into()))?;

    let mut changed = false;
    for (event, matcher) in [
        ("PostToolUse", Some(POST_TOOL_USE_MATCHER)),
        ("Stop", None),
        ("SessionStart", None),
    ] {
        let groups = hooks
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| {
                crate::Error::Other(format!("settings.json: hooks.{event} is not an array"))
            })?;
        if has_our_hook(groups) {
            continue;
        }
        let mut group = Map::new();
        if let Some(m) = matcher {
            group.insert("matcher".into(), m.into());
        }
        group.insert(
            "hooks".into(),
            json!([{ "type": "command", "command": HOOK_COMMAND }]),
        );
        groups.push(Value::Object(group));
        changed = true;
    }

    if !changed {
        return Ok(false);
    }
    write_json(&path, settings, report)
}

/// Merge `mcpServers.featureforge` into `.mcp.json`, preserving other servers.
fn install_mcp_json(repo_root: &Path, mcp_bin_path: &Path, report: &mut KitReport) -> Result<bool> {
    let path = repo_root.join(".mcp.json");
    let mut root = read_json_object(&path)?;

    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| crate::Error::Other(".mcp.json: \"mcpServers\" is not an object".into()))?;

    let desired = json!({
        "command": mcp_bin_path.to_string_lossy(),
        "args": [],
    });
    if servers.get("featureforge") == Some(&desired) {
        return Ok(false);
    }
    servers.insert("featureforge".into(), desired);
    write_json(&path, root, report)
}

/// Append or replace the marker-delimited FeatureForge section of CLAUDE.md.
fn install_claude_md(repo_root: &Path, report: &mut KitReport) -> Result<bool> {
    let path = repo_root.join("CLAUDE.md");
    let section = format!("{CLAUDE_MD_START}\n{CLAUDE_MD_BODY}\n{CLAUDE_MD_END}");
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };

    let updated = match (existing.find(CLAUDE_MD_START), existing.find(CLAUDE_MD_END)) {
        (Some(start), Some(end)) if end >= start => {
            let after = end + CLAUDE_MD_END.len();
            format!("{}{}{}", &existing[..start], section, &existing[after..])
        }
        _ if existing.is_empty() => format!("{section}\n"),
        _ => format!("{}\n{section}\n", existing.trim_end()),
    };
    write_if_changed(&path, &updated, report)
}

/// `.featureforge/` scaffold: hooks/forward.sh (0755), .gitignore, dirs.
fn install_scaffold(repo_root: &Path, report: &mut KitReport) -> Result<bool> {
    let ff = repo_root.join(".featureforge");
    let mut changed = !ff.exists();
    std::fs::create_dir_all(ff.join("runtime"))?;
    std::fs::create_dir_all(ff.join("docs"))?;

    let forward = ff.join("hooks").join("forward.sh");
    changed |= write_if_changed(&forward, FORWARD_SH, report)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&forward, perms)?;
    }

    let gitignore = ff.join(".gitignore");
    match std::fs::read_to_string(&gitignore) {
        Ok(text) if text.lines().any(|l| l.trim() == "runtime/") => {}
        Ok(text) => {
            let updated = format!("{}\nruntime/\n", text.trim_end());
            changed |= write_if_changed(&gitignore, &updated, report)?;
        }
        Err(_) => {
            changed |= write_if_changed(&gitignore, "runtime/\n", report)?;
        }
    }
    Ok(changed)
}
