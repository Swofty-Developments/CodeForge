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

use crate::kit_assets::{
    CLAUDE_MD_BODY, CLAUDE_MD_END, CLAUDE_MD_START, FORWARD_SH, HOOK_COMMAND, HOOK_MARKER,
    POST_TOOL_USE_MATCHER,
};
use crate::Result;

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

/// Where CLAUDE.md's FeatureForge markers stand — a named state, not a guess.
enum MarkerState {
    /// Exactly one ordered START…END pair: replace the section in place.
    Healthy { start: usize, end: usize },
    /// No markers at all: append (or seed an empty file).
    Absent,
    /// Lone, reversed, or duplicated markers: the section cannot be located
    /// safely, so we repair (strip + re-append one clean section) rather than
    /// blindly appending and duplicating on every re-install.
    Corrupt(&'static str),
}

fn marker_state(text: &str) -> MarkerState {
    if text.matches(CLAUDE_MD_START).count() > 1 || text.matches(CLAUDE_MD_END).count() > 1 {
        return MarkerState::Corrupt("duplicate FeatureForge markers");
    }
    match (text.find(CLAUDE_MD_START), text.find(CLAUDE_MD_END)) {
        (None, None) => MarkerState::Absent,
        (Some(start), Some(end)) if end >= start => MarkerState::Healthy { start, end },
        (Some(_), Some(_)) => MarkerState::Corrupt("end marker precedes start marker"),
        _ => MarkerState::Corrupt("a start/end marker is missing its pair"),
    }
}

/// Strip every FeatureForge-marked region (and any stray marker) so a corrupt
/// file converges to exactly one clean section once re-appended.
fn strip_ff_markers(text: &str) -> String {
    let mut out = text.to_string();
    while let (Some(start), Some(end)) = (out.find(CLAUDE_MD_START), out.find(CLAUDE_MD_END)) {
        if end < start {
            break;
        }
        let after = end + CLAUDE_MD_END.len();
        out = format!("{}{}", &out[..start], &out[after..]);
    }
    out.replace(CLAUDE_MD_START, "").replace(CLAUDE_MD_END, "")
}

/// Append or replace the marker-delimited FeatureForge section of CLAUDE.md.
/// Explicit about each marker state so re-installs never duplicate the section.
fn install_claude_md(repo_root: &Path, report: &mut KitReport) -> Result<bool> {
    let path = repo_root.join("CLAUDE.md");
    let section = format!("{CLAUDE_MD_START}\n{CLAUDE_MD_BODY}\n{CLAUDE_MD_END}");
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };

    let append_to = |base: &str| {
        if base.trim().is_empty() {
            format!("{section}\n")
        } else {
            format!("{}\n{section}\n", base.trim_end())
        }
    };
    let updated = match marker_state(&existing) {
        MarkerState::Healthy { start, end } => {
            let after = end + CLAUDE_MD_END.len();
            format!("{}{}{}", &existing[..start], section, &existing[after..])
        }
        MarkerState::Absent => append_to(&existing),
        MarkerState::Corrupt(reason) => {
            tracing::warn!(
                reason,
                path = %path.display(),
                "CLAUDE.md FeatureForge markers are corrupt; repairing"
            );
            append_to(&strip_ff_markers(&existing))
        }
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
