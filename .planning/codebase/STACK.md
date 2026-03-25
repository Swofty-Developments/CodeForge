# Stack

## Language
- Rust (stable, 1.93+)

## Core Dependencies (planned)
| Crate | Version | Purpose |
|-------|---------|---------|
| iced | 0.13+ | Native GUI framework (Elm architecture) |
| rusqlite | latest | SQLite database |
| tokio | 1.x | Async runtime |
| serde | 1.x | Serialization |
| serde_json | 1.x | JSON parsing (JSON-RPC) |
| syntect | 5.x | Syntax highlighting |
| tracing | 0.1 | Structured logging |
| tracing-subscriber | 0.3 | Log output |
| toml | 0.8 | Config file parsing |
| directories | 5.x | XDG directory paths |
| uuid | 1.x | ID generation |
| chrono | 0.4 | Timestamps |

## Build
- `cargo build` / `cargo build --release`
- Dev script: `./dev.sh`

## Platform
- Linux x86_64 (primary)
- Linux aarch64 (secondary)
