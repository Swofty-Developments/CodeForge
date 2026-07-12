# CodeForge

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

scripts/dev.sh
```

`scripts/dev.sh` starts the Vite dev server on **:5183** (override with
`CODEFORGE_DEV_PORT`; :5183 avoids anything already on :5173) plus `tauri dev`
with hot reload. Re-running it replaces the previous instance; `scripts/stop.sh`
stops everything.

## Build a standalone app

```bash
scripts/app.sh
```

Builds the release bundle and opens `CodeForge.app` — self-contained, no dev
server. (Manual equivalent: `cd crates/tauri-app && frontend/node_modules/.bin/tauri
build --bundles app`.)

The Tauri CLI ships with the frontend dev dependencies, so no global install is needed.

## Requirements

- Claude Code CLI installed and authenticated (`claude` on PATH)
- Node.js 18+
- Rust 1.75+
- Tauri CLI v2 — bundled as a frontend dev dependency (or `cargo install tauri-cli`)

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
