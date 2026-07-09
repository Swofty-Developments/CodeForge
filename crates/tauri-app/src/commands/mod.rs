//! Tauri IPC commands, one module per domain. Convention: every command returns
//! `Result<T, String>` (errors formatted with `format!("{e:#}")` for anyhow
//! chains); IDs and paths cross IPC as strings.

pub mod features;
pub mod repo;
pub mod sessions;
pub mod terminals;
pub mod timeline;
pub mod worktrees;
