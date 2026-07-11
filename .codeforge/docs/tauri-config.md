# Tauri Config

## Purpose

Central configuration for the Tauri desktop app shell, defining product identity, window behavior, bundled resources (including the agent-sidecar Node.js runtime), and build hooks that coordinate frontend + backend compilation.

## How it works

- **Product identity**: `identifier` (`net.swofty.codeforge`) and `productName` uniquely identify the app on each platform; version is managed via workspace in `Cargo.toml`.
- **Window settings**: single main window with overlay title bar (hidden title, custom drag region), 1280×820 default size, 800×500 minimum; `withGlobalTauri: true` exposes the Tauri API to frontend JS.
- **Build coordination**: `devUrl` points to the Vite dev server (`localhost:5173`); `beforeBuildCommand` installs frontend deps and runs `npm run build` in `./frontend` before bundling; `beforeDevCommand` is empty (dev server started separately).
- **Plugin manifest**: `Cargo.toml` lists Tauri plugins (`dialog`, `shell`, `process`) that enable file pickers, external process spawning, and lifecycle control; these are consumed by Rust IPC commands.
- **Bundled resources**: agent-sidecar Node.js runtime (`index.mjs`, `node_modules`, `package.json`) is packaged into the app bundle so embedded Claude sessions can spawn the sidecar without requiring a system Node install.
- **Security**: CSP is explicitly set to `null`, trusting the frontend build's own CSP headers.

## Key files

- `crates/tauri-app/tauri.conf.json` — main Tauri configuration (app identity, window, build hooks, bundle resources).
- `crates/tauri-app/Cargo.toml` — declares Tauri plugin dependencies and links to workspace crates.
- `frontend/` — the React/Vite frontend built and bundled into `frontendDist`.
- `agent-sidecar/` — Node.js runtime bundled as a resource for embedded Claude sessions.

## Invariants & gotchas

- **Identifier is permanent**: `net.swofty.codeforge` is the platform identity; changing it breaks app updates and user data migration on macOS/Windows.
- **Agent-sidecar must stay bundled**: the `bundle.resources` wildcard for `agent-sidecar/node_modules/**/*` is required; without it, the sidecar cannot spawn embedded sessions (it's not a dev-only dependency).
- **Window minima are UX floor**: 800×500 is the smallest the app can shrink; going below this breaks sidebar + detail layout on small screens.
- **devUrl must match Vite**: if `frontend/vite.config.ts` changes ports, update `build.devUrl` to match or dev mode will fail to load.
- **CSP null is intentional**: the frontend build (via Vite) injects its own CSP meta tag; Tauri's `csp: null` avoids double-CSP conflicts. Do not set a CSP here unless frontend build also changes.
