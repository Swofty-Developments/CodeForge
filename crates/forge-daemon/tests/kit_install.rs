//! Integration-kit installer tests: idempotence + user-content preservation.

use std::collections::BTreeMap;
use std::path::Path;

use forge_daemon::install_kit;

const MCP_BIN: &str = "/Users/test/.codeforge/bin/forge-mcp";

/// Every kit-managed file, keyed by repo-relative path.
fn kit_files(repo: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for rel in [
        ".claude/settings.json",
        ".mcp.json",
        "CLAUDE.md",
        ".codeforge/hooks/forward.sh",
        ".codeforge/.gitignore",
    ] {
        if let Ok(text) = std::fs::read_to_string(repo.join(rel)) {
            out.insert(rel.to_string(), text);
        }
    }
    out
}

#[test]
fn install_twice_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();

    let first = install_kit(repo, Path::new(MCP_BIN)).unwrap();
    assert!(first.hooks_installed);
    assert!(first.mcp_registered);
    assert!(first.claude_md_updated);
    assert!(first.scaffold_created);
    assert!(!first.changed_files.is_empty());
    let snapshot = kit_files(repo);
    assert_eq!(snapshot.len(), 5, "all kit files should exist");

    let second = install_kit(repo, Path::new(MCP_BIN)).unwrap();
    assert!(!second.hooks_installed);
    assert!(!second.mcp_registered);
    assert!(!second.claude_md_updated);
    assert!(!second.scaffold_created);
    assert!(second.changed_files.is_empty());
    assert_eq!(kit_files(repo), snapshot, "second install must not change files");

    // No duplicated hook groups.
    let settings: serde_json::Value =
        serde_json::from_str(&snapshot[".claude/settings.json"]).unwrap();
    for event in ["PostToolUse", "Stop", "SessionStart"] {
        let groups = settings["hooks"][event].as_array().unwrap();
        let ours = groups
            .iter()
            .filter(|g| g.to_string().contains(".codeforge/hooks/forward.sh"))
            .count();
        assert_eq!(ours, 1, "exactly one codeforge group for {event}");
    }
}

#[test]
fn settings_merge_preserves_user_hooks_and_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    let settings_path = repo.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    let user_settings = serde_json::json!({
        "model": "opus",
        "permissions": { "allow": ["Bash(ls:*)"] },
        "hooks": {
            "PostToolUse": [
                { "matcher": "Bash", "hooks": [{ "type": "command", "command": "echo user-hook" }] }
            ],
            "PreToolUse": [
                { "matcher": "*", "hooks": [{ "type": "command", "command": "echo pre" }] }
            ]
        }
    });
    std::fs::write(&settings_path, user_settings.to_string()).unwrap();

    install_kit(repo, Path::new(MCP_BIN)).unwrap();
    let merged: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();

    assert_eq!(merged["model"], "opus");
    assert_eq!(merged["permissions"]["allow"][0], "Bash(ls:*)");
    assert_eq!(merged["hooks"]["PreToolUse"], user_settings["hooks"]["PreToolUse"]);

    let post = merged["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(post.len(), 2, "user group + codeforge group");
    assert_eq!(post[0], user_settings["hooks"]["PostToolUse"][0]);
    assert_eq!(post[1]["matcher"], "Edit|Write|MultiEdit|NotebookEdit|Bash");
    assert_eq!(
        post[1]["hooks"][0]["command"],
        "\"$CLAUDE_PROJECT_DIR\"/.codeforge/hooks/forward.sh"
    );
    assert!(merged["hooks"]["Stop"].is_array());
    assert!(merged["hooks"]["SessionStart"].is_array());

    // Re-install: still exactly one codeforge PostToolUse group.
    install_kit(repo, Path::new(MCP_BIN)).unwrap();
    let again: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    assert_eq!(again["hooks"]["PostToolUse"].as_array().unwrap().len(), 2);
}

#[test]
fn mcp_json_merge_preserves_other_servers() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    std::fs::write(
        repo.join(".mcp.json"),
        r#"{ "mcpServers": { "playwright": { "command": "npx", "args": ["playwright-mcp"] } } }"#,
    )
    .unwrap();

    install_kit(repo, Path::new(MCP_BIN)).unwrap();
    let merged: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(repo.join(".mcp.json")).unwrap()).unwrap();
    assert_eq!(merged["mcpServers"]["playwright"]["command"], "npx");
    assert_eq!(merged["mcpServers"]["codeforge"]["command"], MCP_BIN);
    assert_eq!(merged["mcpServers"]["codeforge"]["args"], serde_json::json!([]));
}

#[test]
fn claude_md_section_appends_then_repairs() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    std::fs::write(repo.join("CLAUDE.md"), "# My project\n\nUser instructions.\n").unwrap();

    install_kit(repo, Path::new(MCP_BIN)).unwrap();
    let text = std::fs::read_to_string(repo.join("CLAUDE.md")).unwrap();
    assert!(text.starts_with("# My project"));
    assert!(text.contains("<!-- codeforge:start -->"));
    assert!(text.contains("<!-- codeforge:end -->"));
    assert!(text.contains("record_note"));

    // Corrupt the managed section: re-install must restore it without touching user text.
    let corrupted = text.replace("record_note", "GONE");
    std::fs::write(repo.join("CLAUDE.md"), &corrupted).unwrap();
    let report = install_kit(repo, Path::new(MCP_BIN)).unwrap();
    assert!(report.claude_md_updated);
    let repaired = std::fs::read_to_string(repo.join("CLAUDE.md")).unwrap();
    assert_eq!(repaired, text);
}

#[test]
fn claude_md_lone_marker_is_repaired_without_duplicating() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    // A lone START marker with no END: the catch-all used to append a second
    // section, duplicating on every re-install. Repair must converge to exactly
    // one well-formed section.
    std::fs::write(
        repo.join("CLAUDE.md"),
        "# Project\n\n<!-- codeforge:start -->\nstale half-section\n",
    )
    .unwrap();

    let report = install_kit(repo, Path::new(MCP_BIN)).unwrap();
    assert!(report.claude_md_updated);
    let text = std::fs::read_to_string(repo.join("CLAUDE.md")).unwrap();
    assert!(text.starts_with("# Project"), "user text preserved");
    assert_eq!(text.matches("<!-- codeforge:start -->").count(), 1);
    assert_eq!(text.matches("<!-- codeforge:end -->").count(), 1);
    assert!(text.contains("record_note"));

    // Re-install is now idempotent (healthy ordered markers → replace in place).
    let second = install_kit(repo, Path::new(MCP_BIN)).unwrap();
    assert!(!second.claude_md_updated);
    let again = std::fs::read_to_string(repo.join("CLAUDE.md")).unwrap();
    assert_eq!(again.matches("<!-- codeforge:start -->").count(), 1);
    assert_eq!(again, text);
}

#[test]
fn claude_md_reversed_markers_are_repaired() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    // END before START — the section can't be located; repair strips both.
    std::fs::write(
        repo.join("CLAUDE.md"),
        "# Project\n<!-- codeforge:end -->\nmiddle\n<!-- codeforge:start -->\n",
    )
    .unwrap();

    install_kit(repo, Path::new(MCP_BIN)).unwrap();
    let text = std::fs::read_to_string(repo.join("CLAUDE.md")).unwrap();
    assert_eq!(text.matches("<!-- codeforge:start -->").count(), 1);
    assert_eq!(text.matches("<!-- codeforge:end -->").count(), 1);
    // Ordered: start precedes end.
    assert!(
        text.find("<!-- codeforge:start -->") < text.find("<!-- codeforge:end -->"),
        "repaired markers must be ordered"
    );
}

#[test]
fn gitignore_ignores_runtime() {
    let tmp = tempfile::tempdir().unwrap();
    install_kit(tmp.path(), Path::new(MCP_BIN)).unwrap();
    let text = std::fs::read_to_string(tmp.path().join(".codeforge/.gitignore")).unwrap();
    assert!(text.lines().any(|l| l.trim() == "runtime/"));
}

#[test]
fn forward_sh_is_valid_posix_sh_and_executable() {
    let tmp = tempfile::tempdir().unwrap();
    install_kit(tmp.path(), Path::new(MCP_BIN)).unwrap();
    let script = tmp.path().join(".codeforge/hooks/forward.sh");

    let status = std::process::Command::new("sh")
        .arg("-n")
        .arg(&script)
        .status()
        .unwrap();
    assert!(status.success(), "sh -n must accept forward.sh");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&script).unwrap().permissions().mode();
        assert_eq!(mode & 0o755, 0o755, "forward.sh must be executable");
    }

    // Running it with no daemon.json must exit 0 (hooks may never block Claude).
    let status = std::process::Command::new("sh")
        .arg(&script)
        .env("CLAUDE_PROJECT_DIR", tmp.path())
        .stdin(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "forward.sh must exit 0 without a daemon");
}
