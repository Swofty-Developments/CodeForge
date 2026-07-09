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
}
