# FeatureForge

An IDE built off of Claude Code — open a repository and see features, not files.

## Features

- **Feature tree** — the sidebar shows what the codebase *does*, not its directory layout
- **Timeline** — append-only event log of everything agents and humans do, live-updating
- **Feature-grouped diff review** — review pending changes grouped by feature, not by file
- **Embedded Claude Code sessions** — real `claude` sessions streamed into the app
- **MCP + hooks integration kit** — installed into any repo you open, so agents consult the
  feature index and every edit lands on the timeline
- **Cold-start feature indexing** — headless `claude -p` decomposes an unknown repo into features

## Quick start

```bash
cd crates/tauri-app/frontend && npm install && cd -
cd crates/tauri-app/agent-sidecar && npm install && cd -

npm run tauri:dev
```

`npm run tauri:dev` starts the Vite dev server (:5173) and `cargo tauri dev` together.
Prefer two terminals? Run `npm run dev` inside `crates/tauri-app/frontend`, then
`cargo tauri dev` from the repo root (`beforeDevCommand` is intentionally empty).

## Requirements

- Claude Code CLI installed and authenticated (`claude` on PATH)
- Node.js 18+
- Rust 1.75+
- `cargo install tauri-cli` (v2) for `cargo tauri dev`

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
