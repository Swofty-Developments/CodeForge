//! Explicit, DB-authoritative session engagement mode.
//!
//! Replaces the old try-resume-catch-fresh cascade: Rust decides the mode once
//! (from the app DB — `sessions.claude_session_id`) and carries it down the
//! sidecar protocol as a named `mode` field. The sidecar sets exactly one of
//! `options.resume` / `options.continue` / neither from this — it never infers
//! resumability from prior-query state or sniffs SDK error strings.

/// The four SDK permission modes selectable at session start (`StartSessionOpts.
/// permission_mode`) and switchable mid-session via `SessionManager::set_mode`.
/// Distinct from [`SessionMode`] (which decides SDK *engagement*: fresh/resume/
/// continue); this is the *permission* posture Claude runs under.
pub(crate) const PERMISSION_MODES: [&str; 4] = ["default", "acceptEdits", "plan", "bypassPermissions"];

/// True when `mode` is one of the four valid SDK permission modes.
pub(crate) fn is_valid_permission_mode(mode: &str) -> bool {
    PERMISSION_MODES.contains(&mode)
}

/// How a `query` engages the Claude Agent SDK session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMode {
    /// A brand-new SDK session (no `resume`, no `continue`).
    Fresh,
    /// Resume a previously-recorded SDK session id (`options.resume`).
    Resume { claude_session_id: String },
    /// Continue the in-process SDK conversation (`options.continue`) — every
    /// query after the first within one live sidecar.
    ContinueInProcess,
}

impl SessionMode {
    /// The wire value stamped into the sidecar `query` command's `mode` field.
    pub(crate) fn wire(&self) -> &'static str {
        match self {
            SessionMode::Fresh => "fresh",
            SessionMode::Resume { .. } => "resume",
            SessionMode::ContinueInProcess => "continue",
        }
    }
}
