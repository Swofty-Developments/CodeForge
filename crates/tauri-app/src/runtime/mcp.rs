//! Keep `~/.featureforge/bin/forge-mcp` current with the built binary.
//!
//! The MCP proxy is registered in every repo's `.mcp.json` pointing at the copy
//! under `~/.featureforge/bin`. On app start we locate the freshly built binary
//! (sibling of the running exe in dev, or a `target/{debug,release}/forge-mcp`
//! while walking up) and copy it over when newer.

use std::path::{Path, PathBuf};

const MCP_BIN: &str = "forge-mcp";

/// Ensure `~/.featureforge/bin/forge-mcp` reflects the built binary; return its
/// path (the path is returned even when no source was found, so kit install can
/// still register it for a later build).
pub fn ensure_mcp_binary() -> anyhow::Result<PathBuf> {
    let dest_dir = home_dir()?.join(".featureforge").join("bin");
    std::fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(MCP_BIN);

    match locate_source() {
        Some(src) if src != dest => copy_if_newer(&src, &dest)?,
        Some(_) => {}
        None if dest.exists() => {
            tracing::warn!(dest = %dest.display(), "forge-mcp source not found; using existing copy");
        }
        None => tracing::warn!("forge-mcp binary not found; MCP tools unavailable until it is built"),
    }
    Ok(dest)
}

fn home_dir() -> anyhow::Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("no HOME/USERPROFILE in environment"))
}

fn locate_source() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    find_mcp_binary(exe.parent()?)
}

/// Look for `forge-mcp` next to `start_dir`, then in any
/// `target/{debug,release}/forge-mcp` found while walking up from `start_dir`.
fn find_mcp_binary(start_dir: &Path) -> Option<PathBuf> {
    let sibling = start_dir.join(MCP_BIN);
    if sibling.is_file() {
        return Some(sibling);
    }
    let mut dir = Some(start_dir);
    while let Some(d) = dir {
        for profile in ["debug", "release"] {
            let cand = d.join("target").join(profile).join(MCP_BIN);
            if cand.is_file() {
                return Some(cand);
            }
        }
        dir = d.parent();
    }
    None
}

fn copy_if_newer(src: &Path, dest: &Path) -> anyhow::Result<()> {
    let stale = match (mtime(src), mtime(dest)) {
        (Some(s), Some(d)) => s > d,
        (Some(_), None) => true,
        _ => false,
    };
    if stale || !dest.exists() {
        std::fs::copy(src, dest)?;
        set_exec(dest)?;
        tracing::info!(src = %src.display(), dest = %dest.display(), "copied forge-mcp");
    }
    Ok(())
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

#[cfg(unix)]
fn set_exec(p: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_exec(_p: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ff-mcp-{label}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_sibling_binary() {
        let dir = tmp("sibling");
        let bin = dir.join(MCP_BIN);
        std::fs::write(&bin, b"#!/bin/sh\n").unwrap();
        assert_eq!(find_mcp_binary(&dir), Some(bin));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn finds_target_debug_while_walking_up() {
        let root = tmp("walkup");
        let debug = root.join("target").join("debug");
        std::fs::create_dir_all(&debug).unwrap();
        let bin = debug.join(MCP_BIN);
        std::fs::write(&bin, b"x").unwrap();
        // Start from a nested dir with no sibling binary.
        let start = root.join("crates").join("tauri-app");
        std::fs::create_dir_all(&start).unwrap();
        assert_eq!(find_mcp_binary(&start), Some(bin));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn returns_none_when_absent() {
        let dir = tmp("absent");
        assert_eq!(find_mcp_binary(&dir), None);
        std::fs::remove_dir_all(&dir).ok();
    }
}
