The Command Palette was not modified this turn — the changes were to session stop/interrupt behavior in SessionPane and related backend code. The palette remains unchanged and the existing doc is still accurate.

---
# Command Palette

## Purpose

Cmd+K launcher (520px modal, 22vh from top, blur(8px) overlay) providing fuzzy search over features, views, and actions. Escape dismisses; arrow keys + Enter navigate and execute.

## How it works

- **Fuzzy subsequence matching**: greedy left-to-right character match (`fuzzyMatch`) with matched characters highlighted in primary color
- **Three command categories**: `action` (open repo, reindex, new session, toggle session pane), `view` (switch to feature/timeline/diff), `feature` (one dynamic entry per indexed feature)
- **Keyboard-driven navigation**: arrow keys cycle selection (modulo wrap), Enter runs the selected command, Escape handled globally by `App.tsx`
- **Auto-scroll selected item**: `createEffect` on `selected()` uses `scrollIntoView({ block: "nearest" })` to keep the highlighted row visible
- **Input resets selection**: typing resets `selected` to index 0 so the top match stays highlighted
- **Click-to-dismiss overlay**: clicking the backdrop closes the palette; inner `.cmd-palette` stops propagation to prevent bubbling

## Key files

- **crates/tauri-app/frontend/src/components/CommandPalette.tsx** — entire palette implementation (fuzzy match, command registry, rendering, navigation)
- **crates/tauri-app/frontend/src/App.tsx:63-92** — global Cmd+K and Escape handlers that toggle `store.paletteOpen`
- **crates/tauri-app/frontend/src/stores/app-store.ts:207-209** — `setPaletteOpen(bool)` state setter
- **crates/tauri-app/frontend/src/stores/data-slice.ts:134-136** — `openFeatureDetail(slug)` switches to feature view and selects the slug

## Invariants & gotchas

- **App.tsx owns the shortcuts**: CommandPalette itself registers no global listeners; Cmd+K/Escape must live in `App.tsx:onKeyDown` to avoid lifecycle races
- **Empty query matches all**: `fuzzyMatch("", text)` returns `[]` (not `null`), so the empty state shows every command
- **Escape priority order**: palette → pending approval denial → session pane close (App.tsx lines 77-90); palette cannot own Escape or the cascade breaks
- **Category-driven dynamic commands**: features come from `store.features`; adding a new category requires updating `baseCmds` and the CSS `.cat-*` classes
- **520px fixed width, 22vh top offset**: design-system constraint (§5.4 per comment); changing these breaks the intended centering and backdrop-blur composition
- **No session-switching commands yet**: despite the feature description mentioning "sessions (focus)", the current `baseCmds` array has no session-focus entries — only new/toggle-pane
