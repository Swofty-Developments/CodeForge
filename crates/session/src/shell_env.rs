//! Resolve the user's interactive shell environment.
//!
//! Desktop-launched apps (Tauri, Electron, etc.) often inherit a minimal
//! environment that lacks PATH entries, LD_LIBRARY_PATH, nvm/fnm shims, etc.
//! This module runs a one-shot shell to capture the real environment and
//! caches it for the lifetime of the process.

use std::collections::HashMap;
use std::sync::OnceLock;

use tracing::{debug, warn};

/// Cached shell environment, resolved once per process.
static SHELL_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Get the resolved shell environment.  The first call resolves it (blocking);
/// subsequent calls return the cached result instantly.
pub fn get() -> &'static HashMap<String, String> {
    SHELL_ENV.get_or_init(resolve_shell_env)
}

/// Apply the resolved shell environment to a [`tokio::process::Command`].
///
/// This merges the resolved env on top of the command's existing environment
/// rather than replacing it wholesale, so Tauri-specific vars are preserved.
pub fn apply(cmd: &mut tokio::process::Command) {
    for (key, value) in get() {
        cmd.env(key, value);
    }
}

/// Resolve the user's login-shell environment by running:
///   $SHELL -l -i -c 'env -0'      (preferred, NUL-separated)
///   $SHELL -l -c 'env'             (fallback)
///
/// On failure, falls back to the current process environment.
fn resolve_shell_env() -> HashMap<String, String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    debug!("Resolving shell environment using: {shell}");

    // Try NUL-separated first (handles values with newlines).
    if let Some(env) = run_shell_env(&shell, &["-l", "-i", "-c", "env -0"], true) {
        debug!("Resolved {} env vars via `env -0`", env.len());
        return env;
    }

    // Fallback: newline-separated.
    if let Some(env) = run_shell_env(&shell, &["-l", "-c", "env"], false) {
        debug!("Resolved {} env vars via `env` (newline-delimited)", env.len());
        return env;
    }

    warn!("Could not resolve shell environment; falling back to process env");
    std::env::vars().collect()
}

fn run_shell_env(
    shell: &str,
    args: &[&str],
    nul_separated: bool,
) -> Option<HashMap<String, String>> {
    let output = std::process::Command::new(shell)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        // Prevent rc files from prompting for input
        .env("TERM", "dumb")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut env = HashMap::new();

    let entries: Box<dyn Iterator<Item = &str>> = if nul_separated {
        Box::new(stdout.split('\0'))
    } else {
        Box::new(stdout.lines())
    };

    for entry in entries {
        if let Some((key, value)) = entry.split_once('=') {
            if !key.is_empty() && !key.contains(|c: char| c.is_whitespace()) {
                env.insert(key.to_string(), value.to_string());
            }
        }
    }

    if env.is_empty() { None } else { Some(env) }
}

/// Resolve the full path to a command using the resolved shell PATH.
/// Returns None if the command cannot be found.
pub fn which(cmd: &str) -> Option<std::path::PathBuf> {
    let env = get();
    let path_var = env.get("PATH")?;
    for dir in std::env::split_paths(path_var) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
