The feature description might be slightly misleading. Based on my reading, this is primarily about the **backend validation and persistence** of user-set colors, plus the frontend **deterministic hash-based color assignment**. I don't see evidence of group-level inheritance. Let me write the doc based on what the code actually does:

# Feature Color Picker

## Purpose

Validates, persists, and assigns stable colors to features for graph nodes and UI elements. User-set colors override the default hash-based palette.

## How it works

- **Backend validation** — `set_color` accepts `#RGB` or `#RRGGBB` hex strings (or `None` to clear). Invalid colors fail loudly; unknown slugs are rejected (no phantom feature creation).
- **Pin on edit** — setting a color implies `pinned: true`, ensuring user choices survive re-indexing. Clearing a color removes the override but keeps the pin.
- **Persistence** — mutations save to disk and append a `FeatureEdited` timeline event. Failed timeline appends surface as caller errors (not silent log lines).
- **Frontend fallback** — `nodeColor(feature)` uses `feature.color` if set; otherwise `hueForSlug` hashes the slug modulo 8 to pick from the Zed accent palette (`GRAPH_PALETTE`), ensuring stable colors across sessions.
- **Usage** — graph view nodes, worktree project badges, and diff review groups all resolve colors via `nodeColor` or `hueForSlug`.

## Key files

- **crates/tauri-app/src/runtime/feature_color.rs** — core mutation (`set_color`), hex validation, unit tests against a real `FeatureIndex`.
- **crates/tauri-app/src/commands/features.rs** — Tauri command wrapper (`set_feature_color`) that acquires locks and appends timeline events.
- **crates/tauri-app/frontend/src/components/graph/colors.ts** — deterministic hue assignment (`hueForSlug` via hash mod 8), `GRAPH_PALETTE` (Zed accents), `nodeColor` resolver.

## Invariants & gotchas

- **Strict validation** — only `#RGB`/`#RRGGBB` hex or `None` allowed. Bad colors fail mutation; never stored silently.
- **No phantom features** — setting color on unknown slug errors; no side-effect feature creation.
- **Pin is sticky** — both setting and clearing a color keep `pinned: true`. Unpinning requires separate `pin_feature(…, false)`.
- **Timeline transactional** — failed append after persist is surfaced as error, signaling incomplete audit log.
- **Hash collision possible** — 8 palette colors mod over all slugs; unrelated features may share colors. User-set colors break collisions.
