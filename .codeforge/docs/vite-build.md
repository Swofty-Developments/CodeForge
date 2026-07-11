The changes are unrelated to the Vite Build feature. The edited files deal with backend staleness polling (Rust) and frontend index status handling (TypeScript stores/IPC), not Vite's build configuration or dev server. The living doc remains accurate.

---
# Vite Build

## Purpose

Vite 6 dev server and bundler for the SolidJS frontend, integrated with Tauri's desktop build pipeline. Runs the dev server on strict port 5173 and outputs production bundles to `dist/` for the Tauri wrapper to embed.

## How it works

- **Vite plugin stack** — `vite-plugin-solid` for SolidJS JSX compilation and HMR, targeting ESNext output.
- **Strict port binding** — dev server locks to port 5173; fails if unavailable (prevents Tauri from connecting to a stale instance).
- **Separate dev/build flow** — `beforeDevCommand` is empty (Vite runs separately via `npm run dev`); `beforeBuildCommand` runs `npm install && npm run build` during Tauri's production bundle.
- **Type-checked builds** — `npm run build` runs `tsc --noEmit` before Vite to catch type errors; `strict: true` enforces full TypeScript safety.
- **Tauri integration** — Tauri's `devUrl` points to `localhost:5173`; production reads from `frontend/dist/` after Vite's build completes.

## Key files

- `crates/tauri-app/frontend/vite.config.ts` — Vite configuration (SolidJS plugin, strictPort, ESNext target).
- `crates/tauri-app/frontend/package.json` — Scripts for `dev` (Vite server), `build` (type-check + bundle).
- `crates/tauri-app/frontend/tsconfig.json` — TypeScript strict mode, SolidJS JSX config, ESNext module resolution.
- `crates/tauri-app/tauri.conf.json` — Tauri build integration (`devUrl`, `frontendDist`, `beforeBuildCommand`).

## Invariants & gotchas

- **Port 5173 must be free** — `strictPort: true` means dev server crashes if the port is in use; Tauri won't fall back.
- **Run Vite separately in dev** — `beforeDevCommand` is intentionally empty; start Vite manually (`npm run dev`) before launching Tauri, or Tauri will fail to load the frontend.
- **Type errors block production builds** — `npm run build` fails on any TypeScript error (`tsc --noEmit`); fix types before bundling.
- **ESNext output only** — Vite doesn't transpile to older targets; assumes a modern Chromium webview (Tauri's embedded browser).
- **Tauri expects `frontend/dist/`** — never change `frontendDist` path without updating Tauri's config; the bundler won't find the assets.
