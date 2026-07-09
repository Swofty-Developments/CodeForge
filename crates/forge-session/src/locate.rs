//! Sidecar script resolution: bundled app locations first, then the dev
//! workspace (walking up from the executable/cwd), then a compile-time fallback.

use std::path::{Path, PathBuf};

use crate::claude::SIDECAR_SCRIPT;

/// Resolve `agent-sidecar/index.mjs`, in priority order:
/// 1. macOS bundle: `Contents/MacOS/<exe>` → `Contents/Resources/agent-sidecar/`
/// 2. beside the executable (Linux/Windows bundles)
/// 3. dev builds: walk up from the exe dir (`target/debug`), then from cwd,
///    looking for `crates/tauri-app/agent-sidecar/index.mjs`
/// 4. compile-time fallback: this crate's manifest dir → workspace root
pub(crate) fn resolve_sidecar() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled = exe_dir.join("../Resources/agent-sidecar/index.mjs");
            if bundled.is_file() {
                return Some(bundled);
            }
            let beside = exe_dir.join("agent-sidecar/index.mjs");
            if beside.is_file() {
                return Some(beside);
            }
            if let Some(found) = find_walking_up(exe_dir) {
                return Some(found);
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(found) = find_walking_up(&cwd) {
            return Some(found);
        }
    }
    let fallback = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(SIDECAR_SCRIPT);
    fallback.is_file().then_some(fallback)
}

/// Walk up from `start` (inclusive, at most 10 parents) looking for the
/// workspace-relative sidecar script. Search root injectable for tests.
pub(crate) fn find_walking_up(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    for _ in 0..=10 {
        let d = dir?;
        let candidate = d.join(SIDECAR_SCRIPT);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = d.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_up_from_injected_root_to_sidecar() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        let sidecar_dir = root.join("crates/tauri-app/agent-sidecar");
        std::fs::create_dir_all(&sidecar_dir).expect("create sidecar dir");
        std::fs::write(sidecar_dir.join("index.mjs"), "// stub").expect("write stub");

        let start = root.join("target/debug");
        std::fs::create_dir_all(&start).expect("create target dir");

        let found = find_walking_up(&start).expect("sidecar found from nested target dir");
        assert_eq!(found, root.join(SIDECAR_SCRIPT));

        // Starting at the root itself also resolves (inclusive walk).
        assert_eq!(find_walking_up(root), Some(root.join(SIDECAR_SCRIPT)));
    }

    #[test]
    fn missing_sidecar_yields_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert_eq!(find_walking_up(tmp.path()), None);
    }

    #[test]
    fn resolves_real_workspace_sidecar() {
        // Dev checkout: tier 3/4 must find crates/tauri-app/agent-sidecar/index.mjs.
        let found = resolve_sidecar().expect("sidecar resolvable in dev workspace");
        assert!(found.ends_with("agent-sidecar/index.mjs"));
        assert!(found.is_file());
    }
}
