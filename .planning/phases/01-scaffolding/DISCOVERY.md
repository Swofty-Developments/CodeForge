# Phase 1 Discovery: Project Scaffolding & Core Architecture

## Research Findings

### iced Framework (0.13+)
- Elm architecture: `Application` trait with `new()`, `update()`, `view()`, `subscription()`
- Async via `Command<Message>` (returns futures that produce Messages)
- `Subscription` for long-running streams (agent output)
- Built-in widgets: `text`, `button`, `text_input`, `scrollable`, `container`, `column`, `row`, `pane_grid`
- Custom themes via `Theme` struct with palette
- `iced::widget::pane_grid` for split pane layouts (sidebar + main)

### Cargo Workspace Structure
- Root `Cargo.toml` with `[workspace]` members
- Three crates: `app`, `session`, `persistence`
- Shared types can go in `session` or a small `types` crate

### Dark Theme Implementation
- iced supports custom `Theme` with `Palette` (background, text, primary, success, danger colors)
- Can customize per-widget appearance via style closures
- Monospace font loading via `iced::Font::with_name()` or embedded bytes

### Agent Communication (from t3code analysis)
- **Codex**: `codex app-server` subprocess, NDJSON stdio, JSON-RPC protocol
  - Flow: spawn → `initialize` → `initialized` → `thread/start` → `turn/start`
  - Approval: `item/commandExecution/requestApproval` (server request requiring response)
  - Events: notifications like `turn/completed`, `content.delta`
- **Claude Code**: For Rust, spawn `claude` CLI with `--print` or use `claude code --json` mode
  - Alternative: spawn `claude` with `--output-format json` for structured output

### SQLite
- `rusqlite` with `bundled` feature for zero system deps
- Sync API — use `tokio::task::spawn_blocking` for async bridge
- Migrations: simple SQL strings applied on startup

## Decisions for This Phase
1. Use Cargo workspace with 3 crates (app, session, persistence)
2. iced 0.13 with custom dark theme
3. Initial app shell: pane_grid with sidebar placeholder + main area placeholder
4. Embed a monospace font (JetBrains Mono or similar, or use system default)
5. dev.sh: simple `cargo run` wrapper
