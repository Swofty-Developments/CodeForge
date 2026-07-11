The changes described in the agent's notes are about staleness detection and indexing — backend changes in `staleness.rs`, `repo_open.rs`, and `state.rs`, plus the frontend store handling `index:status` events. These have **no impact** on the Graph View feature, which visualizes features and their relationships through force-directed simulation. The Graph View is purely a visualization layer over `store.features` and doesn't interact with indexing/staleness logic.

The existing doc remains accurate and complete.

---
# Graph View

## Purpose
Interactive force-directed graph visualizing features as nodes and their relationships as edges. Node size scales with file count; edges connect features via shared files or entry-point references. Users drag to reposition, click to select, and assign custom colors.

## How it works
- **Simulation physics**: Hand-rolled force integration (no external libs) with repulsion between all node pairs, spring forces along edges (rest length inversely proportional to edge weight), and weak gravity toward the center. Alpha cools exponentially until kinetic energy drops below threshold.
- **Topology caching**: Graph rebuilds only when features are added/removed or file membership changes — not on renames or recolors. Prior node positions are preserved across rebuilds via `posCache` keyed by slug.
- **Reduced-motion**: If `prefers-reduced-motion` is set, the simulation settles synchronously (240 ticks in one pass) instead of animating via `requestAnimationFrame`.
- **Drag interaction**: Pointer-down sets `fx`/`fy` to pin the node at cursor position; pointer-move updates the pin; pointer-up clears it. Drag distance under 4px is treated as a click (selects the node).
- **Edge weight**: Edges are created for every shared file (weight += 1) and for entry-point linkage (if feature A's entry point is a file in feature B, an edge A↔B with weight += 1).
- **Color palette**: Default node color is hashed from slug into the Zed One Dark accent set; user-assigned colors (via the popover picker) override.

## Key files
- **crates/tauri-app/frontend/src/views/GraphView.tsx** — main component: drives the sim loop, handles drag/click, renders SVG nodes/edges, shows popover on selection.
- **crates/tauri-app/frontend/src/components/graph/graph-sim.ts** — force-directed physics: `buildGraph` creates nodes/edges from features, `step` integrates repulsion/springs/gravity for one tick.
- **crates/tauri-app/frontend/src/components/graph/NodePopover.tsx** — side panel for selected node: displays name/tags/stats, color swatch picker, "Open feature detail" hand-off.
- **crates/tauri-app/frontend/src/components/graph/colors.ts** — deterministic hue assignment (`hueForSlug` via hash mod), Zed accent palette, `nodeColor` resolver (explicit color or fallback).

## Invariants & gotchas
- **Position cache keyed by slug**: renaming a feature (slug change) drops its cached position and triggers a full reset on next rebuild. The slug is the stable identity; changing it breaks continuity.
- **`fx`/`fy` must be null when not dragging**: pinned nodes skip velocity integration. Forgetting to clear `fx`/`fy` on pointer-up freezes the node permanently.
- **Alpha decay and reheat**: `reheat()` resets `alpha = 1` and restarts the loop. If you mutate the graph mid-animation without reheating, nodes drift without enough energy to settle.
- **Reduced-motion synchronous settle**: the 240-tick loop in `reheat()` must complete before `setFrame` fires or positions are stale. Do not interleave async work during reduced-motion settling.
- **Edge weight determines rest length**: `restLength = max(58, 150 - weight * 16)` — higher weight pulls nodes closer. If you cap edge weight or change the formula, node spacing will change.
- **SVG coordinate transform**: `toSvg(e: PointerEvent)` must use `getScreenCTM().inverse()` or drag coordinates are wrong when the SVG is scaled/translated by CSS.
