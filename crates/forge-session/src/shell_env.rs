//! Login-shell environment resolution.
//!
//! Desktop-launched apps (Finder/dock) get a minimal environment; without this,
//! `node`/`claude` are not on PATH (nvm/fnm shims) and spawns fail or pick a
//! wrong binary with mismatched shared libraries. Resolved once per process,
//! merged **over** the existing command env (never replacing it wholesale).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Cached shell environment, resolved once per process.
static SHELL_ENV: OnceLock<ShellEnv> = OnceLock::new();

/// The user's environment for spawning child tools, plus whether it was
/// genuinely resolved from the login shell. `Unresolved` is a NAMED state —
/// both shell probes failed, so only the minimal process env is available and
/// PATH-dependent lookups (`node`/`claude`) may miss; callers surface why.
pub enum ShellEnv {
    Resolved(HashMap<String, String>),
    Unresolved(HashMap<String, String>),
}

impl ShellEnv {
    /// The underlying variable map (present in both states).
    pub fn vars(&self) -> &HashMap<String, String> {
        match self {
            ShellEnv::Resolved(m) | ShellEnv::Unresolved(m) => m,
        }
    }

    /// True only when resolved from the login shell (not the fallback).
    pub fn is_resolved(&self) -> bool {
        matches!(self, ShellEnv::Resolved(_))
    }
}

/// The user's login-shell environment state (cached).
pub fn get() -> &'static ShellEnv {
    SHELL_ENV.get_or_init(resolve_shell_env)
}

/// Whether the login-shell environment was resolved (vs. the process-env
/// fallback). Lets a downstream `node`/`claude` miss explain its cause.
pub fn is_resolved() -> bool {
    get().is_resolved()
}

/// Merges the resolved env on top of the command's existing environment
/// rather than replacing it wholesale, so Tauri-specific vars are preserved.
pub fn apply(cmd: &mut tokio::process::Command) {
    for (key, value) in get().vars() {
        cmd.env(key, value);
    }
}

/// Find `cmd` on the resolved PATH (login-shell PATH, not process PATH).
pub fn which(cmd: &str) -> Option<PathBuf> {
    let env = get().vars();
    let path = env.get("PATH").cloned().or_else(|| std::env::var("PATH").ok())?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Locate the Claude Code CLI: the resolved PATH first, then the well-known
/// install locations PATH misses when the login-shell probe fails or the
/// profile doesn't export them — the native installer (`~/.local/bin`), the
/// legacy local install (`~/.claude/local`), and Homebrew (Apple-silicon and
/// Intel prefixes).
pub fn locate_claude() -> Option<PathBuf> {
    if let Some(found) = which("claude") {
        return Some(found);
    }
    well_known_claude_paths().into_iter().find(|p| is_executable(p))
}

/// Candidate `claude` install locations outside PATH, best-first.
fn well_known_claude_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if let Some(home) = home {
        let home = PathBuf::from(home);
        candidates.push(home.join(".local/bin/claude"));
        candidates.push(home.join(".claude/local/claude"));
    }
    candidates.push(PathBuf::from("/opt/homebrew/bin/claude"));
    candidates.push(PathBuf::from("/usr/local/bin/claude"));
    candidates
}

/// A regular file with an exec bit (symlinks followed, so Homebrew's Cellar
/// links qualify). Non-unix: existence is the best available signal.
#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &std::path::Path) -> bool {
    path.is_file()
}

/// Resolve the user's login-shell environment by running:
///   `$SHELL -l -i -c 'env -0'`  (preferred, NUL-separated)
///   `$SHELL -l -c 'env'`        (fallback)
/// If BOTH probes fail this returns [`ShellEnv::Unresolved`] over the process
/// env — flagged, not silently passed off as a resolved login shell.
fn resolve_shell_env() -> ShellEnv {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    if let Some(env) = run_shell_env(&shell, &["-l", "-i", "-c", "env -0"], true) {
        return ShellEnv::Resolved(env);
    }
    if let Some(env) = run_shell_env(&shell, &["-l", "-c", "env"], false) {
        return ShellEnv::Resolved(env);
    }
    tracing::warn!(
        shell = %shell,
        "shell-env-unresolved: both login-shell probes failed; using the minimal process \
         environment (PATH-dependent spawns like node/claude may not be found)"
    );
    ShellEnv::Unresolved(std::env::vars().collect())
}

fn run_shell_env(shell: &str, args: &[&str], nul_separated: bool) -> Option<HashMap<String, String>> {
    let output = std::process::Command::new(shell)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        // Prevent rc files from prompting for input.
        .env("TERM", "dumb")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let entries: Vec<&str> = if nul_separated {
        raw.split('\0').collect()
    } else {
        raw.lines().collect()
    };

    let mut env = HashMap::new();
    for entry in entries {
        if let Some((key, value)) = entry.split_once('=') {
            if key.is_empty() || key.chars().any(char::is_whitespace) {
                continue;
            }
            env.insert(key.to_string(), value.to_string());
        }
    }

    if env.is_empty() {
        None
    } else {
        Some(env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_some_env() {
        let env = get();
        assert!(!env.vars().is_empty());
        assert!(env.vars().contains_key("PATH") || std::env::var("PATH").is_ok());
    }

    #[test]
    fn which_finds_sh() {
        assert!(which("sh").is_some());
    }

    #[cfg(unix)]
    #[test]
    fn executable_probe_requires_exec_bit() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let plain = tmp.path().join("claude");
        std::fs::write(&plain, "#!/bin/sh\n").expect("write");
        std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        assert!(!is_executable(&plain), "no exec bit");
        std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        assert!(is_executable(&plain), "exec bit set");
        assert!(!is_executable(&tmp.path().join("missing")), "missing file");
    }

    #[test]
    fn well_known_paths_cover_native_and_homebrew() {
        let paths = well_known_claude_paths();
        let rendered: Vec<String> =
            paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        assert!(rendered.iter().any(|p| p.ends_with(".local/bin/claude")), "native installer");
        assert!(rendered.iter().any(|p| p.ends_with(".claude/local/claude")), "legacy local");
        assert!(rendered.contains(&"/opt/homebrew/bin/claude".to_string()), "brew arm64");
        assert!(rendered.contains(&"/usr/local/bin/claude".to_string()), "brew intel");
    }
}
