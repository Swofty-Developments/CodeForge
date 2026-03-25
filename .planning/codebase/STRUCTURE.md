# Directory Structure (Planned)

```
codeforge/
├── Cargo.toml              # Workspace root
├── dev.sh                  # Development launcher
├── crates/
│   ├── app/                # Main iced application
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs     # Entry point
│   │       ├── app.rs      # App struct, update, view
│   │       ├── message.rs  # Message enum
│   │       ├── state.rs    # Application state
│   │       ├── theme.rs    # Dark IDE theme
│   │       ├── views/
│   │       │   ├── sidebar.rs
│   │       │   ├── chat.rs
│   │       │   ├── composer.rs
│   │       │   ├── tabs.rs
│   │       │   └── settings.rs
│   │       ├── widgets/
│   │       │   ├── markdown.rs
│   │       │   └── code_block.rs
│   │       └── subscriptions/
│   │           └── agent.rs
│   ├── session/            # Agent session management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manager.rs  # Session lifecycle
│   │       ├── claude.rs   # Claude Code adapter
│   │       ├── codex.rs    # Codex adapter
│   │       ├── protocol.rs # JSON-RPC types
│   │       └── types.rs    # Session types
│   └── persistence/        # SQLite layer
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── db.rs       # Connection management
│           ├── migrations.rs
│           ├── models.rs   # Data models
│           └── queries.rs  # CRUD operations
└── assets/
    └── fonts/              # Monospace fonts
```
