The edited files are unrelated to Graph View — they concern Tauri command registration, CLI retry logic, and headless indexing. The existing doc remains accurate.

---
# Graph View

## Purpose

Force-directed graph visualization of the feature set — nodes represent features (sized by file count), edges represent shared files or entry-point linkage. Click to select a feature (opens a side popover for color customization), drag to reposition, or reset the layout.

## How it works

- `buildGraph` constructs the topology: nodes per feature (radius ~ `sqrt(fileCount)`), edges weighted by shared-file count and entry-point containment. Graph rebuilds only when features are added/removed or file membership changes — name/color updates don't reset layout.
- `step` integrates one physics tick: repulsion (all node pairs), edge springs (rest length inversely proportional to weight), and center gravity, all multiplied by a cooling `alpha`. Returns kinetic energy so the loop can stop when settled.
- `tick` runs the simulation via `requestAnimationFrame`, cooling `alpha *= 0.985` each frame and halting when `alpha < 0.02` and `energy < 0.01`.
- Under `prefers-reduced-motion: reduce`, reheats settle synchronously in 240 ticks (no continuous animation). During a drag, `nudge()` only triggers a repaint without reheating.
- Node drag sets `fx`/`fy` (pinned position) and reheats the sim. On release, clears `fx`/`fy` unless the pointer moved less than 4px (threshold for a click vs. drag).
- `NodePopover` displays feature details (name, tags, file/entry counts) and a color swatch picker (`GRAPH_PALETTE` + an "Auto" clear button) that calls `appStore.setFeatureColor`.

## Key files

- **GraphView.tsx** — main view, renders SVG nodes/edges, drives the physics loop, handles drag/select interaction.
- **graph-sim.ts** — graph builder (`buildGraph` constructs nodes/edges from features) and physics integrator (`step` applies forces and updates positions).
- **NodePopover.tsx** — side popover for the selected node (color picker, stats, hand-off to feature detail).
- **colors.ts** — `GRAPH_PALETTE` (Zed One Dark accent swatches) and `hueForSlug` (deterministic hash so slugs get stable default colors).

## Invariants & gotchas

- **Position cache survives graph rebuilds.** `posCache` is keyed by slug, so recolors/renames preserve node positions. Clearing the cache triggers a full layout reset.
- **`fx`/`fy` pins during drag.** A node with `fx !== null` or `fy !== null` has zero velocity and position set exactly to `(fx, fy)` — clearing these is mandatory on drag release.
- **Click-vs-drag threshold is 4px.** Pointer movement less than `hypot(Δx, Δy) < 4` between down and up is treated as a click (selects the feature), not a drag.
- **Reduced motion settles synchronously.** The 240-tick settle happens in one frame — updating the sim mid-settle breaks convergence. During a drag under reduced motion, only repaint (`setFrame`), don't reheat.
- **Edge weight drives rest length.** Higher weight → shorter spring rest length → tighter clusters. Don't invert the weight/distance relationship without also tuning `SPRING` constant.
- **Graph rebuilds fire a reheat.** `createEffect(() => { graph(); reheat(); })` ensures topology changes (new/removed features) always restart the simulation from `alpha = 1`.
