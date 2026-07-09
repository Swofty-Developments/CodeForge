//! Embedded PTY terminal subsystem (FZ-3).
//!
//! One [`portable_pty`] pseudo-terminal per terminal, owned by the
//! [`TerminalManager`] that lives in `AppState`. A blocking reader thread streams
//! each PTY's output to the frontend on the `terminal:data` channel (output bytes
//! are **base64-encoded**; xterm decodes them). Child exit is reported on
//! `terminal:exit`. Input keystrokes arrive as raw **UTF-8** and are written
//! straight to the PTY master.
//!
//! Teardown is explicit, never best-effort: `close_terminal` kills the child (its
//! reader thread then hits EOF and ends), and [`TerminalManager`]'s `Drop` kills
//! every remaining child so closing the app leaves no orphan shells.

mod manager;
mod pty;

pub use manager::TerminalManager;

use serde::Serialize;

/// Tauri event carrying a chunk of PTY output. `data` is base64-encoded raw bytes.
pub const TERMINAL_DATA: &str = "terminal:data";
/// Tauri event emitted once, when a terminal's child process exits.
pub const TERMINAL_EXIT: &str = "terminal:exit";

/// A live terminal as seen by `list_terminals` (FZ-3).
#[derive(Debug, Clone, Serialize)]
pub struct TerminalInfo {
    /// Opaque terminal id (uuid v4).
    pub id: String,
    /// The working directory the shell was spawned in (the active worktree).
    pub cwd: String,
    /// Human-readable label (the cwd's final path component).
    pub title: String,
}

/// Payload for [`TERMINAL_DATA`]: `data` is base64-encoded PTY output.
#[derive(Debug, Clone, Serialize)]
pub struct TerminalData {
    pub id: String,
    pub data: String,
}

/// Payload for [`TERMINAL_EXIT`]. `code` is the child's exit code, or `None` when
/// the code could not be reaped (a named "unknown" state, not a silent zero).
#[derive(Debug, Clone, Serialize)]
pub struct TerminalExit {
    pub id: String,
    pub code: Option<i32>,
}

/// Terminal subsystem errors.
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("terminal not found: {0}")]
    NotFound(String),
    #[error("pty error: {0}")]
    Pty(String),
    #[error("terminal write failed: {0}")]
    Write(#[source] std::io::Error),
    #[error("terminal manager lock poisoned")]
    LockPoisoned,
}
