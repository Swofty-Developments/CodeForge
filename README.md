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

npm run tauri:dev
```

`npm run tauri:dev` starts the Vite dev server (:5173) and `cargo tauri dev` together.
Prefer two terminals? Run `npm run dev` inside `crates/tauri-app/frontend`, then
`cargo tauri dev` from the repo root (`beforeDevCommand` is intentionally empty).

If port 5173 is taken, change `server.port` in `crates/tauri-app/frontend/vite.config.ts`
and `build.devUrl` in `crates/tauri-app/tauri.conf.json` to match.

## Build a standalone app

```bash
cd crates/tauri-app
frontend/node_modules/.bin/tauri build --bundles app
open target/release/bundle/macos/CodeForge.app
```

The Tauri CLI ships with the frontend dev dependencies, so no global install is needed.

## Requirements

- Claude Code CLI installed and authenticated (`claude` on PATH)
- Node.js 18+
- Rust 1.75+
- Tauri CLI v2 — bundled as a frontend dev dependency (or `cargo install tauri-cli`)

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
