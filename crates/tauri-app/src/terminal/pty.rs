//! Low-level PTY process: spawn a shell in a pseudo-terminal, stream its output
//! through a [`TerminalSink`], and expose write/resize/kill on the master side.
//!
//! This module is Tauri-free: output and exit are delivered to a [`TerminalSink`]
//! so the reader plumbing can be exercised headless (the manager's `AppSink`
//! bridges to Tauri events; tests use a channel sink).

use std::io::{Read, Write};

use base64::Engine as _;
use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize};

use super::TerminalError;

/// Initial PTY geometry. The frontend issues a `resize_terminal` as soon as the
/// xterm view mounts, so these are only the pre-measurement dimensions.
const INITIAL_COLS: u16 = 80;
const INITIAL_ROWS: u16 = 24;
/// Reader chunk size; PTY output is drained in blocks of this many bytes.
const READ_CHUNK: usize = 8192;

/// Receives a terminal's output and its eventual exit. `data` chunks are
/// base64-encoded raw PTY bytes; `code` is the child exit code (`None` = the
/// code could not be reaped, a named "unknown", not a silent zero).
pub trait TerminalSink: Send + 'static {
    fn data(&self, id: &str, chunk: &str);
    fn exit(&self, id: &str, code: Option<i32>);
}

/// A shell running in a PTY. Holds the master side for write/resize plus a killer
/// handle; the child itself is owned by the reader thread, which reaps it on exit.
pub struct PtyProcess {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

impl PtyProcess {
    /// Spawn `shell` in a fresh PTY rooted at `cwd`, wiring a blocking reader
    /// thread that pushes output to `sink` and reports the exit code once. `id`
    /// is the caller's terminal id, stamped on every sink call.
    pub fn spawn(
        sink: Box<dyn TerminalSink>,
        id: String,
        cwd: &str,
        shell: &str,
    ) -> Result<Self, TerminalError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: INITIAL_ROWS,
                cols: INITIAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::Pty(format!("openpty failed: {e:#}")))?;

        let cmd = build_command(cwd, shell);
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::Pty(format!("failed to spawn shell {shell}: {e:#}")))?;

        // Drop the slave so only the child holds it; otherwise the master reader
        // would never see EOF after the child exits.
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::Pty(format!("clone reader failed: {e:#}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::Pty(format!("take writer failed: {e:#}")))?;
        let killer = child.clone_killer();

        spawn_reader(sink, id, reader, child);

        Ok(Self { master: pair.master, writer, killer })
    }

    /// Write raw UTF-8 keystrokes to the PTY master.
    pub fn write(&mut self, data: &[u8]) -> Result<(), TerminalError> {
        self.writer.write_all(data).map_err(TerminalError::Write)?;
        self.writer.flush().map_err(TerminalError::Write)
    }

    /// Resize the PTY window (SIGWINCH to the child).
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), TerminalError> {
        self.master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| TerminalError::Pty(format!("resize failed: {e:#}")))
    }

    /// Kill the child. The reader thread then hits EOF, reaps it, and reports the
    /// exit through the sink before ending.
    pub fn kill(&mut self) {
        if let Err(e) = self.killer.kill() {
            tracing::debug!("terminal kill failed (already exited?): {e}");
        }
    }
}

/// Build the shell command: cwd = the active worktree, env enriched with the
/// login-shell environment (so PATH/nvm shims resolve) and a real `TERM` so the
/// shell and TUIs render colors.
fn build_command(cwd: &str, shell: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new(shell);
    cmd.cwd(cwd);
    for (key, value) in forge_session::shell_env::get().vars() {
        cmd.env(key, value);
    }
    cmd.env("TERM", "xterm-256color");
    cmd
}

/// Spawn the blocking reader thread. It owns `child` so it can reap the exit
/// code once the PTY closes.
fn spawn_reader(
    sink: Box<dyn TerminalSink>,
    id: String,
    mut reader: Box<dyn Read + Send>,
    mut child: Box<dyn Child + Send + Sync>,
) {
    std::thread::spawn(move || {
        let mut buf = [0u8; READ_CHUNK];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF: the child closed its end of the PTY.
                Ok(n) => {
                    let chunk = base64::engine::general_purpose::STANDARD.encode(&buf[..n]);
                    sink.data(&id, &chunk);
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    // A closed PTY surfaces as EIO on some platforms rather than EOF.
                    tracing::debug!(terminal = %id, "pty read ended: {e}");
                    break;
                }
            }
        }

        let code = match child.wait() {
            Ok(status) => Some(status.exit_code() as i32),
            Err(e) => {
                tracing::error!(terminal = %id, "failed to reap terminal child: {e}");
                None
            }
        };
        sink.exit(&id, code);
    });
}
