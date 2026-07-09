//! Integration-kit installer. Runs on every repo open; **idempotent** and never
//! clobbers user content:
//!
//! - `.claude/settings.json` — merge hooks: PostToolUse (Edit|Write|MultiEdit|
//!   NotebookEdit), Stop, SessionStart → `.featureforge/hooks/forward.sh`
//!   (reads port from daemon.json, `curl --max-time 2 ... || true`).
//! - `.mcp.json` — merge a `featureforge` stdio server → `~/.featureforge/bin/forge-mcp`.
//! - `CLAUDE.md` — append a marker-delimited section
//!   (`<!-- featureforge:start -->` … `<!-- featureforge:end -->`) telling agents
//!   to consult the feature index via MCP before exploring and to `record_note`
//!   decisions. Re-installs replace only the marked section.
//! - `.featureforge/` scaffold + `.featureforge/.gitignore` ignoring `runtime/`.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::Result;

/// Markers delimiting the FeatureForge-owned section of CLAUDE.md.
pub const CLAUDE_MD_START: &str = "<!-- featureforge:start -->";
pub const CLAUDE_MD_END: &str = "<!-- featureforge:end -->";

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
    let _ = (repo_root, mcp_bin_path);
    // IMPLEMENT(agent): idempotent merge of .claude/settings.json hooks,
    // .mcp.json server entry, CLAUDE.md marker section, .featureforge scaffold
    // (features.json, docs/, runtime/, .gitignore, hooks/forward.sh chmod +x).
    todo!("install_kit")
}
