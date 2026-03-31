//! Git hook management: enumeration, installation, and execution.
//!
//! Provides types for representing git hooks, their scripts, and
//! operations for installing, removing, and running hooks with timeouts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Standard git hook types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GitHookType {
    /// Runs before a commit message is created.
    PreCommit,
    /// Runs to prepare the default commit message.
    PrepareCommitMsg,
    /// Runs after the commit message is entered, before commit.
    CommitMsg,
    /// Runs after a commit is created.
    PostCommit,
    /// Runs before `git push` sends data.
    PrePush,
    /// Runs before a rebase begins.
    PreRebase,
    /// Runs after a successful checkout / switch.
    PostCheckout,
    /// Runs after a successful merge.
    PostMerge,
    /// Runs before `git rewrite` operations (amend, rebase).
    PreAutoGc,
    /// Runs on the server before updating refs.
    PreReceive,
    /// Runs on the server to validate each ref update.
    Update,
    /// Runs on the server after all refs are updated.
    PostReceive,
    /// Runs after `git applypatch` completes.
    PostApplypatch,
    /// Runs before `git applypatch` starts.
    PreApplypatch,
    /// Runs when `git push` is used to update a work tree.
    PostRewrite,
}

impl GitHookType {
    /// Return the filename expected in the hooks directory.
    pub fn filename(&self) -> &'static str {
        match self {
            GitHookType::PreCommit => "pre-commit",
            GitHookType::PrepareCommitMsg => "prepare-commit-msg",
            GitHookType::CommitMsg => "commit-msg",
            GitHookType::PostCommit => "post-commit",
            GitHookType::PrePush => "pre-push",
            GitHookType::PreRebase => "pre-rebase",
            GitHookType::PostCheckout => "post-checkout",
            GitHookType::PostMerge => "post-merge",
            GitHookType::PreAutoGc => "pre-auto-gc",
            GitHookType::PreReceive => "pre-receive",
            GitHookType::Update => "update",
            GitHookType::PostReceive => "post-receive",
            GitHookType::PostApplypatch => "post-applypatch",
            GitHookType::PreApplypatch => "pre-applypatch",
            GitHookType::PostRewrite => "post-rewrite",
        }
    }

    /// Return a human-readable description of when this hook fires.
    pub fn description(&self) -> &'static str {
        match self {
            GitHookType::PreCommit => "Runs before creating a commit",
            GitHookType::PrepareCommitMsg => "Prepares the default commit message",
            GitHookType::CommitMsg => "Validates or modifies the commit message",
            GitHookType::PostCommit => "Runs after a commit is created",
            GitHookType::PrePush => "Runs before push sends data to remote",
            GitHookType::PreRebase => "Runs before a rebase begins",
            GitHookType::PostCheckout => "Runs after a successful checkout",
            GitHookType::PostMerge => "Runs after a successful merge",
            GitHookType::PreAutoGc => "Runs before automatic garbage collection",
            GitHookType::PreReceive => "Server-side: runs before updating refs",
            GitHookType::Update => "Server-side: validates each ref update",
            GitHookType::PostReceive => "Server-side: runs after all refs updated",
            GitHookType::PostApplypatch => "Runs after applying a patch",
            GitHookType::PreApplypatch => "Runs before applying a patch",
            GitHookType::PostRewrite => "Runs after rewriting commits",
        }
    }

    /// Return all standard hook types.
    pub fn all() -> &'static [GitHookType] {
        &[
            GitHookType::PreCommit,
            GitHookType::PrepareCommitMsg,
            GitHookType::CommitMsg,
            GitHookType::PostCommit,
            GitHookType::PrePush,
            GitHookType::PreRebase,
            GitHookType::PostCheckout,
            GitHookType::PostMerge,
            GitHookType::PreAutoGc,
            GitHookType::PreReceive,
            GitHookType::Update,
            GitHookType::PostReceive,
            GitHookType::PostApplypatch,
            GitHookType::PreApplypatch,
            GitHookType::PostRewrite,
        ]
    }

    /// Return whether this is a client-side hook.
    pub fn is_client_side(&self) -> bool {
        !matches!(
            self,
            GitHookType::PreReceive | GitHookType::Update | GitHookType::PostReceive
        )
    }
}

impl fmt::Display for GitHookType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.filename())
    }
}

/// A hook script that can be installed into the hooks directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookScript {
    /// The hook type this script implements.
    pub hook_type: GitHookType,
    /// The script content (bash, python, etc).
    pub content: String,
    /// The interpreter line (e.g., "#!/bin/bash").
    pub shebang: String,
    /// Whether this hook should be executable.
    pub executable: bool,
    /// Optional description comment added to the script.
    pub description: Option<String>,
}

impl HookScript {
    /// Create a new bash hook script.
    pub fn bash(hook_type: GitHookType, content: impl Into<String>) -> Self {
        Self {
            hook_type,
            content: content.into(),
            shebang: "#!/bin/bash".to_string(),
            executable: true,
            description: None,
        }
    }

    /// Create a new shell hook script with sh.
    pub fn sh(hook_type: GitHookType, content: impl Into<String>) -> Self {
        Self {
            hook_type,
            content: content.into(),
            shebang: "#!/bin/sh".to_string(),
            executable: true,
            description: None,
        }
    }

    /// Add a description to the hook.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Generate the full script content including shebang and description.
    pub fn render(&self) -> String {
        let mut script = String::new();
        script.push_str(&self.shebang);
        script.push('\n');
        if let Some(ref desc) = self.description {
            script.push_str("# ");
            script.push_str(desc);
            script.push('\n');
        }
        script.push_str("# Auto-generated by CodeForge\n");
        script.push('\n');
        script.push_str(&self.content);
        if !script.ends_with('\n') {
            script.push('\n');
        }
        script
    }

    /// Return the target file path within the hooks directory.
    pub fn target_path(&self, hooks_dir: &Path) -> PathBuf {
        hooks_dir.join(self.hook_type.filename())
    }
}

/// The result of running a hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookExecResult {
    /// The hook that was executed.
    pub hook_type: GitHookType,
    /// The exit code (0 = success).
    pub exit_code: i32,
    /// Standard output from the hook.
    pub stdout: String,
    /// Standard error output from the hook.
    pub stderr: String,
    /// How long the hook took to run.
    pub duration_ms: u64,
    /// Whether the hook was killed due to timeout.
    pub timed_out: bool,
}

impl HookExecResult {
    /// Whether the hook succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }
}

impl fmt::Display for HookExecResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.timed_out {
            write!(f, "{}: timed out after {}ms", self.hook_type, self.duration_ms)
        } else if self.success() {
            write!(f, "{}: ok ({}ms)", self.hook_type, self.duration_ms)
        } else {
            write!(
                f,
                "{}: failed (exit {}, {}ms)",
                self.hook_type, self.exit_code, self.duration_ms
            )
        }
    }
}

/// Configuration for hook execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRunConfig {
    /// Maximum time to wait for a hook to complete.
    pub timeout: Duration,
    /// Environment variables to pass to the hook.
    pub env: HashMap<String, String>,
    /// Working directory for hook execution.
    pub cwd: Option<PathBuf>,
    /// Whether to capture stdout/stderr.
    pub capture_output: bool,
}

impl Default for HookRunConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            env: HashMap::new(),
            cwd: None,
            capture_output: true,
        }
    }
}

/// Trait for managing git hooks in a repository.
pub trait HookManager {
    /// The error type for hook operations.
    type Error: std::error::Error;

    /// List all installed hooks.
    fn list_hooks(&self) -> Result<Vec<GitHookType>, Self::Error>;

    /// Check if a specific hook is installed.
    fn has_hook(&self, hook_type: GitHookType) -> Result<bool, Self::Error>;

    /// Read the content of an installed hook.
    fn read_hook(&self, hook_type: GitHookType) -> Result<Option<String>, Self::Error>;

    /// Install a hook script.
    fn install_hook(&self, script: &HookScript) -> Result<(), Self::Error>;

    /// Remove a hook.
    fn remove_hook(&self, hook_type: GitHookType) -> Result<(), Self::Error>;

    /// Execute a hook with the given configuration.
    fn run_hook(
        &self,
        hook_type: GitHookType,
        args: &[&str],
        config: &HookRunConfig,
    ) -> Result<HookExecResult, Self::Error>;
}

/// Status of a hook in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookStatus {
    /// The hook type.
    pub hook_type: GitHookType,
    /// Whether the hook file exists.
    pub installed: bool,
    /// Whether the hook file is executable.
    pub executable: bool,
    /// Size of the hook file in bytes.
    pub size_bytes: Option<u64>,
    /// The interpreter from the shebang line.
    pub interpreter: Option<String>,
}

impl fmt::Display for HookStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.installed {
            let exec = if self.executable { "executable" } else { "not executable" };
            write!(f, "{}: installed ({})", self.hook_type, exec)
        } else {
            write!(f, "{}: not installed", self.hook_type)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_filenames() {
        assert_eq!(GitHookType::PreCommit.filename(), "pre-commit");
        assert_eq!(GitHookType::PostMerge.filename(), "post-merge");
    }

    #[test]
    fn script_rendering() {
        let script = HookScript::bash(GitHookType::PreCommit, "cargo fmt --check")
            .with_description("Format check");
        let rendered = script.render();
        assert!(rendered.starts_with("#!/bin/bash"));
        assert!(rendered.contains("Format check"));
        assert!(rendered.contains("cargo fmt --check"));
    }

    #[test]
    fn client_side_hooks() {
        assert!(GitHookType::PreCommit.is_client_side());
        assert!(!GitHookType::PreReceive.is_client_side());
    }
}
