/* Feature graph (capability E) — an interactive force-directed SVG over the
 * feature set. Nodes = features (radius ~ file count), edges = shared files /
 * entry-point links. Drag to reposition, click to select (side popover with a
 * color picker), reset layout, legend. Self-contained: the sim is hand-rolled
 * (graph-sim.ts) and driven by requestAnimationFrame, honoring reduced-motion. */

import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { appStore } from "../stores/app-store";
import type { Feature } from "../types";
import { buildGraph, step, type Graph, type SimBounds } from "../components/graph/graph-sim";
import { nodeColor } from "../components/graph/colors";
import { NodePopover } from "../components/graph/NodePopover";

const BOUNDS: SimBounds = { width: 1000, height: 680 };
const reduceMotion = () =>
  typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

export function GraphView() {
  const { store } = appStore;
  const [frame, setFrame] = createSignal(0);
  const [hoverSlug, setHoverSlug] = createSignal<string | null>(null);
  const [dragIndex, setDragIndex] = createSignal<number | null>(null);
  let svgRef: SVGSVGElement | undefined;
  const posCache = new Map<string, { x: number; y: number }>();

  const featureBySlug = createMemo(() => new Map(store.features.map((f) => [f.slug, f])));

  // Rebuilds only on topology change (graph-sim reads slug/files/entryPoints, not
  // name/color) — carrying prior positions so recolors never reset the layout.
  const graph = createMemo<Graph>(() => {
    const g = buildGraph(store.features, BOUNDS);
    for (const n of g.nodes) {
      const c = posCache.get(n.slug);
      if (c) { n.x = c.x; n.y = c.y; }
    }
    return g;
  });

  const positions = createMemo(() => {
    frame();
    return graph().nodes.map((n) => ({ x: n.x, y: n.y }));
  });

  const selected = createMemo<Feature | null>(() => {
    const slug = store.selectedFeature;
    if (!slug) return null;
    const f = featureBySlug().get(slug);
    return f && graph().nodes.some((n) => n.slug === slug) ? f : null;
  });

  // ── Simulation loop ────────────────────────────────────────────────────────
  let alpha = 1;
  let raf = 0;
  let running = false;

  function tick(): void {
    const g = graph();
    const energy = step(g, BOUNDS, alpha);
    for (const n of g.nodes) posCache.set(n.slug, { x: n.x, y: n.y });
    alpha *= 0.985;
    setFrame((f) => f + 1);
    if (dragIndex() !== null || (alpha > 0.02 && energy > 0.01)) {
      raf = requestAnimationFrame(tick);
    } else {
      running = false;
    }
  }

  function reheat(): void {
    alpha = 1;
    if (reduceMotion()) {
      // Settle synchronously, no continuous animation.
      const g = graph();
      for (let k = 0; k < 240; k++) step(g, BOUNDS, alpha *= 0.985);
      for (const n of g.nodes) posCache.set(n.slug, { x: n.x, y: n.y });
      setFrame((f) => f + 1);
      return;
    }
    if (!running) { running = true; raf = requestAnimationFrame(tick); }
  }

  // During a drag we only need to repaint; a full re-settle each mousemove would
  // be a jank source under reduced motion.
  function nudge(): void {
    if (reduceMotion()) setFrame((f) => f + 1);
    else reheat();
  }

  onMount(() => reheat());
  onCleanup(() => cancelAnimationFrame(raf));
  // Reheat when the topology changes (new/removed features).
  createEffect(() => { graph(); reheat(); });

  // ── Pointer interaction ────────────────────────────────────────────────────
  let interacted = false;
  let dragStart = { x: 0, y: 0 };
  let dragMoved = false;

  function toSvg(e: PointerEvent): { x: number; y: number } {
    if (!svgRef) return { x: 0, y: 0 };
    const pt = svgRef.createSVGPoint();
    pt.x = e.clientX;
    pt.y = e.clientY;
    const m = svgRef.getScreenCTM();
    if (!m) return { x: 0, y: 0 };
    const p = pt.matrixTransform(m.inverse());
    return { x: p.x, y: p.y };
  }

  function onNodeDown(i: number, e: PointerEvent): void {
    e.stopPropagation();
    interacted = true;
    dragMoved = false;
    setDragIndex(i);
    const p = toSvg(e);
    dragStart = p;
    const n = graph().nodes[i];
    n.fx = p.x; n.fy = p.y;
    svgRef?.setPointerCapture(e.pointerId);
    nudge();
  }

  function onMove(e: PointerEvent): void {
    const i = dragIndex();
    if (i === null) return;
    const p = toSvg(e);
    const n = graph().nodes[i];
    n.fx = p.x; n.fy = p.y; n.x = p.x; n.y = p.y;
    if (Math.hypot(p.x - dragStart.x, p.y - dragStart.y) > 4) dragMoved = true;
    nudge();
  }

  function onUp(): void {
    const i = dragIndex();
    if (i !== null) {
      const n = graph().nodes[i];
      n.fx = null; n.fy = null;
      if (!dragMoved) appStore.selectFeature(n.slug);
      setDragIndex(null);
      nudge();
    }
  }

  function onBgClick(): void {
    if (interacted) { interacted = false; return; }
    appStore.selectFeature(null);
  }

  function resetLayout(): void {
    posCache.clear();
    const fresh = buildGraph(store.features, BOUNDS);
    const g = graph();
    g.nodes.forEach((n, i) => {
      n.x = fresh.nodes[i].x; n.y = fresh.nodes[i].y;
      n.vx = 0; n.vy = 0; n.fx = null; n.fy = null;
    });
    appStore.selectFeature(null);
    reheat();
  }

  return (
    <div class="gv">
      <Show
        when={store.features.length > 0}
        fallback={
          <div class="gv-empty">
            <div class="gv-empty-title">No feature graph yet</div>
            <div class="gv-empty-sub">Index this repository to map features and their shared files.</div>
            <button class="gv-empty-btn" onClick={() => void appStore.reindex()}>Index repository</button>
          </div>
        }
      >
        <svg
          ref={svgRef}
          class="gv-svg"
          viewBox={`0 0 ${BOUNDS.width} ${BOUNDS.height}`}
          preserveAspectRatio="xMidYMid meet"
          onPointerMove={onMove}
          onPointerUp={onUp}
          onClick={onBgClick}
        >
          <g class="gv-edges">
            <For each={graph().edges}>
              {(e) => {
                const pa = () => positions()[e.a];
                const pb = () => positions()[e.b];
                const conn = () => {
                  const s = store.selectedFeature;
                  return !!s && (graph().nodes[e.a].slug === s || graph().nodes[e.b].slug === s);
                };
                return (
                  <line
                    x1={pa().x} y1={pa().y} x2={pb().x} y2={pb().y}
                    stroke={conn() ? "var(--primary)" : "var(--border-strong)"}
                    stroke-width={Math.min(3, 0.6 + e.weight * 0.5)}
                    stroke-opacity={conn() ? 0.7 : 0.4}
                  />
                );
              }}
            </For>
          </g>

          <For each={graph().nodes}>
            {(n, i) => {
              const feat = () => featureBySlug().get(n.slug);
              const color = () => { const f = feat(); return f ? nodeColor(f) : "#878a98"; };
              const isSel = () => store.selectedFeature === n.slug;
              const isHov = () => hoverSlug() === n.slug;
              const p = () => positions()[i()];
              return (
                <g
                  class="gv-node"
                  transform={`translate(${p().x},${p().y})`}
                  onPointerDown={(e) => onNodeDown(i(), e)}
                  onMouseEnter={() => setHoverSlug(n.slug)}
                  onMouseLeave={() => setHoverSlug(null)}
                >
                  <circle
                    r={n.r}
                    fill={color()}
                    fill-opacity={isSel() || isHov() ? 0.92 : 0.6}
                    stroke={color()}
                    stroke-width={isSel() ? 2.5 : 1}
                  />
                  <text y={n.r + 13} text-anchor="middle" class="gv-label" classList={{ "gv-label--on": isSel() || isHov() }}>
                    {feat()?.name ?? n.slug}
                  </text>
                </g>
              );
            }}
          </For>
        </svg>

        <Show when={selected()}>
          {(f) => (
            <NodePopover
              feature={f()}
              onOpenDetail={() => appStore.openFeatureDetail(f().slug)}
              onClose={() => appStore.selectFeature(null)}
            />
          )}
        </Show>

        <div class="gv-legend">
          <div class="gv-legend-row"><span class="gv-legend-dot gv-legend-dot--sm" /><span class="gv-legend-dot gv-legend-dot--lg" /> node = feature (size ~ files)</div>
          <div class="gv-legend-row"><span class="gv-legend-line" /> edge = shared files / entry links</div>
        </div>
        <button class="gv-reset" title="Reset layout" onClick={resetLayout}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 12a9 9 0 1 0 3-6.7L3 8m0-5v5h5" />
          </svg>
          Reset layout
        </button>
      </Show>

      <style>{`
        .gv { flex: 1; position: relative; min-height: 0; overflow: hidden; background: var(--bg-base); }
        .gv-svg { width: 100%; height: 100%; display: block; touch-action: none; }
        .gv-edges line { transition: stroke 0.15s; }
        .gv-node { cursor: pointer; }
        .gv-node circle { transition: fill-opacity 0.15s, stroke-width 0.12s; }
        .gv-label { font-family: var(--font-body); font-size: 12px; font-weight: 500; fill: var(--text-tertiary); pointer-events: none; user-select: none; }
        .gv-label--on { fill: var(--text); }
        .gv-empty { position: absolute; inset: 0; margin: auto; height: fit-content; width: fit-content; display: flex; flex-direction: column; align-items: center; gap: 6px; text-align: center; }
        .gv-empty-title { font-size: 15px; font-weight: 600; color: var(--text-secondary); }
        .gv-empty-sub { font-size: 12px; color: var(--text-tertiary); max-width: 280px; line-height: 1.5; }
        .gv-empty-btn { margin-top: 10px; padding: 7px 16px; font-size: 12.5px; font-weight: 600; color: #16202e; background: var(--primary); border-radius: var(--radius-md); }
        .gv-empty-btn:hover { filter: brightness(1.08); }
        .gv-legend { position: absolute; left: 12px; bottom: 12px; z-index: 4; display: flex; flex-direction: column; gap: 5px; padding: 8px 10px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-md); font-size: 10.5px; color: var(--text-tertiary); }
        .gv-legend-row { display: flex; align-items: center; gap: 6px; }
        .gv-legend-dot { border-radius: 50%; background: var(--primary); opacity: 0.65; flex-shrink: 0; }
        .gv-legend-dot--sm { width: 7px; height: 7px; }
        .gv-legend-dot--lg { width: 12px; height: 12px; }
        .gv-legend-line { width: 16px; height: 2px; background: var(--border-strong); flex-shrink: 0; }
        .gv-reset { position: absolute; right: 12px; bottom: 12px; z-index: 4; display: inline-flex; align-items: center; gap: 6px; padding: 6px 11px; font-size: 11.5px; font-weight: 500; color: var(--text-secondary); background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-md); }
        .gv-reset:hover { background: var(--bg-accent); color: var(--text); }
      `}</style>
    </div>
  );
}
