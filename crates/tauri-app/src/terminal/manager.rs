//! [`TerminalManager`]: owns every live PTY, keyed by terminal id. Lives in
//! `AppState`. Teardown is explicit — `close` kills the child, and `Drop` kills
//! every remaining child so closing the app leaves no orphan shells.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter};

use super::pty::{PtyProcess, TerminalSink};
use super::{TerminalData, TerminalError, TerminalExit, TerminalInfo, TERMINAL_DATA, TERMINAL_EXIT};

/// One live terminal: its shell PTY plus the metadata `list_terminals` reports.
struct Terminal {
    cwd: String,
    title: String,
    pty: PtyProcess,
}

/// Owns all spawned PTYs. Interior-mutable so it can sit directly in `AppState`
/// without an outer lock; a poisoned map lock is a named error, never silently
/// swallowed into an empty result.
pub struct TerminalManager {
    inner: Mutex<HashMap<String, Terminal>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self { inner: Mutex::new(HashMap::new()) }
    }

    /// Open a terminal, streaming its output to Tauri events on `app`. Returns
    /// the new terminal id (uuid v4).
    pub fn open(
        &self,
        app: AppHandle,
        cwd: String,
        shell: Option<String>,
    ) -> Result<String, TerminalError> {
        self.open_with_sink(Box::new(AppSink(app)), cwd, shell)
    }

    /// Sink-generic open used by [`open`](Self::open) (Tauri sink) and tests
    /// (channel sink). Resolves the shell, spawns the PTY, and records it.
    fn open_with_sink(
        &self,
        sink: Box<dyn TerminalSink>,
        cwd: String,
        shell: Option<String>,
    ) -> Result<String, TerminalError> {
        let shell = resolve_shell(shell);
        let id = uuid::Uuid::new_v4().to_string();
        let title = title_for(&cwd);
        let pty = PtyProcess::spawn(sink, id.clone(), &cwd, &shell)?;
        let mut map = self.inner.lock().map_err(|_| TerminalError::LockPoisoned)?;
        map.insert(id.clone(), Terminal { cwd, title, pty });
        Ok(id)
    }

    /// Write raw UTF-8 keystrokes to a terminal. Unknown id → [`TerminalError::NotFound`].
    pub fn write(&self, id: &str, data: &[u8]) -> Result<(), TerminalError> {
        let mut map = self.inner.lock().map_err(|_| TerminalError::LockPoisoned)?;
        let term = map.get_mut(id).ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        term.pty.write(data)
    }

    /// Resize a terminal's PTY. Unknown id → [`TerminalError::NotFound`].
    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<(), TerminalError> {
        let map = self.inner.lock().map_err(|_| TerminalError::LockPoisoned)?;
        let term = map.get(id).ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        term.pty.resize(cols, rows)
    }

    /// Kill and forget a terminal. Unknown id → [`TerminalError::NotFound`].
    pub fn close(&self, id: &str) -> Result<(), TerminalError> {
        let mut term = {
            let mut map = self.inner.lock().map_err(|_| TerminalError::LockPoisoned)?;
            map.remove(id).ok_or_else(|| TerminalError::NotFound(id.to_string()))?
        };
        term.pty.kill();
        Ok(())
    }

    /// Snapshot of every live terminal.
    pub fn list(&self) -> Result<Vec<TerminalInfo>, TerminalError> {
        let map = self.inner.lock().map_err(|_| TerminalError::LockPoisoned)?;
        Ok(map
            .iter()
            .map(|(id, t)| TerminalInfo { id: id.clone(), cwd: t.cwd.clone(), title: t.title.clone() })
            .collect())
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        // Kill every child even through a poisoned lock — no orphan shells.
        let mut map = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        for term in map.values_mut() {
            term.pty.kill();
        }
        map.clear();
    }
}

/// Bridges PTY output to Tauri events. `data` payloads carry base64 output;
/// `exit` fires once per terminal.
struct AppSink(AppHandle);

impl TerminalSink for AppSink {
    fn data(&self, id: &str, chunk: &str) {
        let _ = self
            .0
            .emit(TERMINAL_DATA, TerminalData { id: id.to_string(), data: chunk.to_string() });
    }
    fn exit(&self, id: &str, code: Option<i32>) {
        let _ = self.0.emit(TERMINAL_EXIT, TerminalExit { id: id.to_string(), code });
    }
}

/// Resolve the shell to spawn (named precedence, not a guess-when-missing):
/// explicit `shell` arg → login-shell `$SHELL` → process `$SHELL` → OS default
/// (`/bin/zsh` on macOS).
fn resolve_shell(shell: Option<String>) -> String {
    if let Some(s) = shell {
        let s = s.trim();
        if !s.is_empty() {
            return s.to_string();
        }
    }
    if let Some(s) = forge_session::shell_env::get().vars().get("SHELL") {
        if !s.is_empty() {
            return s.clone();
        }
    }
    if let Ok(s) = std::env::var("SHELL") {
        if !s.is_empty() {
            return s;
        }
    }
    default_shell()
}

#[cfg(target_os = "macos")]
fn default_shell() -> String {
    "/bin/zsh".to_string()
}

#[cfg(not(target_os = "macos"))]
fn default_shell() -> String {
    "/bin/sh".to_string()
}

/// Terminal label = the cwd's final path component (the full path if it has none,
/// e.g. `/`).
fn title_for(cwd: &str) -> String {
    Path::new(cwd)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use std::sync::mpsc::{channel, Sender};
    use std::time::{Duration, Instant};

    /// Test sink: base64-decodes output chunks and forwards the raw bytes.
    struct ChannelSink(Sender<Vec<u8>>);

    impl TerminalSink for ChannelSink {
        fn data(&self, _id: &str, chunk: &str) {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(chunk)
                .expect("sink emits valid base64");
            let _ = self.0.send(bytes);
        }
        fn exit(&self, _id: &str, _code: Option<i32>) {}
    }

    struct NullSink;
    impl TerminalSink for NullSink {
        fn data(&self, _id: &str, _chunk: &str) {}
        fn exit(&self, _id: &str, _code: Option<i32>) {}
    }

    fn temp_cwd() -> String {
        std::env::temp_dir().to_string_lossy().into_owned()
    }

    /// portable-pty allocates a real pty, so this runs headless: write a command,
    /// read its echoed output back.
    #[test]
    fn echo_roundtrip() {
        let mgr = TerminalManager::new();
        let (tx, rx) = channel::<Vec<u8>>();
        let id = mgr
            .open_with_sink(Box::new(ChannelSink(tx)), temp_cwd(), Some("sh".to_string()))
            .expect("open terminal");

        mgr.write(&id, b"echo hi\n").expect("write");

        let mut acc = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(chunk) => {
                    acc.extend_from_slice(&chunk);
                    if String::from_utf8_lossy(&acc).contains("hi") {
                        break;
                    }
                }
                Err(_) => continue,
            }
        }

        let out = String::from_utf8_lossy(&acc).into_owned();
        mgr.close(&id).expect("close");
        assert!(out.contains("hi"), "expected 'hi' in pty output, got: {out:?}");
    }

    #[test]
    fn resize_live_terminal() {
        let mgr = TerminalManager::new();
        let id = mgr
            .open_with_sink(Box::new(NullSink), temp_cwd(), Some("sh".to_string()))
            .expect("open");
        mgr.resize(&id, 120, 40).expect("resize");
        mgr.close(&id).expect("close");
    }

    #[test]
    fn close_removes_from_list() {
        let mgr = TerminalManager::new();
        let id = mgr
            .open_with_sink(Box::new(NullSink), temp_cwd(), Some("sh".to_string()))
            .expect("open");
        assert_eq!(mgr.list().unwrap().len(), 1);
        mgr.close(&id).expect("close");
        assert!(mgr.list().unwrap().is_empty());
    }

    #[test]
    fn unknown_id_is_named_error() {
        let mgr = TerminalManager::new();
        assert!(matches!(mgr.write("nope", b"x"), Err(TerminalError::NotFound(_))));
        assert!(matches!(mgr.resize("nope", 80, 24), Err(TerminalError::NotFound(_))));
        assert!(matches!(mgr.close("nope"), Err(TerminalError::NotFound(_))));
    }
}
