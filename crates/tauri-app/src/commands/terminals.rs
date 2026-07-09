//! Embedded terminal IPC commands (FZ-3). Each PTY is owned by the
//! `TerminalManager` in `AppState`. Output streams on the `terminal:data` event
//! (base64-encoded bytes; the frontend decodes) and `terminal:exit` fires once
//! per terminal on child exit. Input (`write_terminal`) is raw UTF-8.

use tauri::State;

use crate::state::AppState;
use crate::terminal::TerminalInfo;

/// Spawn the user's shell in a PTY rooted at `cwd` (the active worktree).
/// `shell` overrides the resolved default. Returns the new terminal id.
#[tauri::command]
pub async fn open_terminal(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    cwd: String,
    shell: Option<String>,
) -> Result<String, String> {
    state.terminals.open(app, cwd, shell).map_err(|e| e.to_string())
}

/// Write raw UTF-8 keystrokes to a terminal.
#[tauri::command]
pub async fn write_terminal(state: State<'_, AppState>, id: String, data: String) -> Result<(), String> {
    state.terminals.write(&id, data.as_bytes()).map_err(|e| e.to_string())
}

/// Resize a terminal's PTY window.
#[tauri::command]
pub async fn resize_terminal(
    state: State<'_, AppState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    state.terminals.resize(&id, cols, rows).map_err(|e| e.to_string())
}

/// Kill a terminal's child and forget it.
#[tauri::command]
pub async fn close_terminal(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.terminals.close(&id).map_err(|e| e.to_string())
}

/// Snapshot of every live terminal.
#[tauri::command]
pub async fn list_terminals(state: State<'_, AppState>) -> Result<Vec<TerminalInfo>, String> {
    state.terminals.list().map_err(|e| e.to_string())
}
