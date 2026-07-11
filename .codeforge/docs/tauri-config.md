The changes described (staleness detection poller) don't relate to Tauri Config at all — they're runtime behavior in Rust modules. The tauri.conf.json file was not edited. The existing doc remains accurate.

---
# Tauri Config

## Purpose

Central Tauri 2.10 application configuration defining window properties, capabilities/permissions, build pipeline, and bundling for the CodeForge desktop app.

## How it works

- **Window defaults**: 1280×820 centered window with overlay title bar (hidden title), minimum 800×500, resizable.
- **Bundle configuration**: targets all platforms, includes icon variants (PNG/ICO), bundles the agent-sidecar Node.js runtime and its dependencies as resources.
- **Dev/build pipeline**: dev server at `localhost:5173`, production builds run `npm install && npm run build` in `frontend/` before bundling.
- **Capabilities**: default permission set grants dialog, shell, process, and core event APIs to all windows via `capabilities/default.json`.
- **Global Tauri API**: `withGlobalTauri: true` exposes the Tauri API object globally in the frontend.

## Key files

- **`crates/tauri-app/tauri.conf.json`** — root config: window geometry, dev/build commands, bundle targets, identifier.
- **`crates/tauri-app/capabilities/default.json`** — permission manifest: grants dialog/shell/process/event APIs to all windows.
- **`crates/tauri-app/build.rs`** — invokes `tauri_build::build()` to generate Rust code from config at compile time.

## Invariants & gotchas

- **Agent sidecar bundling**: the `agent-sidecar/` directory (Node.js MCP server) must be listed in `bundle.resources` or the runtime won't find it.
- **Minimum dimensions**: 800×500 enforced — UI must remain usable at this size; test responsive layouts.
- **CSP disabled**: `"csp": null` allows arbitrary inline scripts/styles; re-enabling CSP will break dev hot-reload and may require nonce plumbing.
- **beforeDevCommand is empty**: frontend dev server must be started separately (not auto-launched by Tauri); deployment scripts must handle this.
- **titleBarStyle "Overlay"**: custom window controls required in frontend; native title bar is hidden.
