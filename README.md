# CodeForge

[<img src="https://discordapp.com/assets/e4923594e694a21542a489471ecffa50.svg" alt="Discord" height="55" />](https://discord.swofty.net)

An AI code editor that wraps Claude Code and Codex into a native desktop app with multi-thread sessions, an embedded browser, and a sleek dark UI.

<img width="1921" height="1078" alt="image" src="https://github.com/user-attachments/assets/0f3f4dc6-f57f-4d02-96ab-d35e70b79c29" />

> **Note**: This project is under active development and is not yet production-ready.

## Features

- **Multi-Thread Sessions** - Run multiple Claude Code / Codex conversations in parallel tabs
- **Real-Time Streaming** - Token-by-token response streaming via Claude's stream-json protocol
- **Model Selector** - Switch between Opus, Sonnet, Haiku per-message
- **Embedded Browser** - CDP screencast-powered browser pane with element inspector for extracting HTML/CSS
- **Diff Editor** - Git-powered diff viewer pane showing all changed files with syntax highlighting
- **File Attachments** - Drag-and-drop files or use the paperclip button to attach context
- **Auto Thread Naming** - Spawns a separate Claude/Codex process to name threads after 3 messages
- **Command Palette** - Cmd+K quick actions
- **Cross-Message Search** - Cmd+Shift+F to search across all threads
- **Usage Dashboard** - Token counts, costs, and model breakdown
- **Split View** - Side-by-side thread comparison
- **Worktree Management** - Git worktree per thread for isolated changes
- **Per-Thread Browser** - Each thread gets its own browser instance with independent state

## Requirements

- macOS / Linux / Windows
- [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated
- Node.js 18+
- Rust 1.75+
- Playwright Chromium (`npx playwright install chromium`)

## Quick Start

```bash
# Clone
git clone https://github.com/Swofty-Developments/CodeForge.git
cd CodeForge

# Install frontend dependencies
cd crates/tauri-app/frontend && npm install && cd ../../..

# Install Playwright browser
cd crates/tauri-app/frontend && npx playwright install chromium && cd ../../..

# Run in dev mode
cargo tauri dev
```

## Testing

```bash
# Playwright E2E tests (21 tests)
cd crates/tauri-app/frontend && npm run test:e2e

# Rust integration tests (real Claude Code)
cargo test -p codeforge-session --test claude_integration
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd+K | Command palette |
| Cmd+Shift+F | Search across messages |
| Cmd+Shift+B | Toggle browser pane |
| Cmd+Shift+D | Toggle diff view |
| Cmd+Shift+U | Usage dashboard |
| Cmd+\ | Split view |
| Enter | Send message |
| Shift+Enter | New line |

## License

See repository for license details.
