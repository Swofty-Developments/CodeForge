# Project: CodeForge

## Vision
A fast, native Linux desktop GUI for AI coding agents (Claude Code and Codex). Built in Rust with iced, providing multi-session management, threaded conversations, project organization, and approval controls — all persisted in SQLite.

## Requirements

### Validated
- Agent sessions: start, stop, interrupt, resume (Claude Code + Codex)
- Streaming chat with markdown + syntax highlighted code blocks
- Thread management with titles and timestamps
- Project grouping by directory path
- Multi-session concurrency via tabs
- Supervised + auto-approve approval modes
- SQLite persistence for threads, messages, sessions
- Dark IDE-like theme
- `./dev.sh` launcher
- Basic settings panel

### Active (Under Discussion)
- Exact subprocess communication protocol for each provider
- Markdown rendering approach in iced (custom widget vs built-in)

### Out of Scope
- Code diffs, embedded terminal, git integration
- File attachments, remote access, auto-update
- Keybinding customization, light theme

## Constraints
- Rust stable toolchain
- Linux x86_64 primary target
- Startup < 500ms, message latency < 50ms
- Binary < 30MB release

## Key Decisions
| Decision | Status | Rationale |
|----------|--------|-----------|
| Rust + iced for GUI | Approved | Native performance, Elm architecture fits reactive UI |
| SQLite via rusqlite | Approved | Embedded, zero-config, matches t3code's approach |
| Subprocess stdio for agents | Approved | Same approach as t3code, avoids API key management |
| tokio for async | Approved | Standard Rust async runtime, needed for subprocess IO |
| Multi-session via tabs | Approved | Power user requirement, each tab = independent session |
