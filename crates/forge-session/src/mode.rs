//! Explicit, DB-authoritative session engagement mode.
//!
//! Replaces the old try-resume-catch-fresh cascade: Rust decides the mode once
//! (from the app DB — `sessions.claude_session_id`) and carries it down the
//! sidecar protocol as a named `mode` field. The sidecar sets exactly one of
//! `options.resume` / `options.continue` / neither from this — it never infers
//! resumability from prior-query state or sniffs SDK error strings.

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
