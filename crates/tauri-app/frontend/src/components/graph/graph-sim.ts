/* Hand-rolled force-directed graph over the feature set — no external libs.
 * Nodes are features (radius ~ file count); edges join two features per shared
 * file (weight = shared-file count) and also link a feature to another whose
 * files contain one of its entry points. The Sim integrates repulsion + edge
 * springs + gravity with velocity damping and a cooling `alpha`. */

import type { Feature } from "../../types";

export interface GNode {
  slug: string;
  fileCount: number;
  r: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
  /** Pinned position while dragging (null = free). */
  fx: number | null;
  fy: number | null;
}

export interface GEdge {
  a: number;
  b: number;
  weight: number;
}

export interface Graph {
  nodes: GNode[];
  edges: GEdge[];
}

export interface SimBounds {
  width: number;
  height: number;
}

function radiusFor(fileCount: number): number {
  return Math.max(9, Math.min(30, 9 + Math.sqrt(fileCount) * 3.2));
}

/** A topology key: rebuild the graph (and reset layout) only when features are
 *  added/removed or their file/entry-point membership changes — NOT on rename or
 *  recolor (those are read live at render time). */
export function topologyKey(features: Feature[]): string {
  return features
    .map((f) => `${f.slug}#${f.files.map((x) => x.path).join(",")}#${f.entryPoints.join(",")}`)
    .join("|");
}

export function buildGraph(features: Feature[], bounds: SimBounds): Graph {
  const cx = bounds.width / 2;
  const cy = bounds.height / 2;
  const ring = Math.min(bounds.width, bounds.height) * 0.32;
  const n = features.length;

  const nodes: GNode[] = features.map((f, i) => {
    const theta = (i / Math.max(1, n)) * Math.PI * 2;
    return {
      slug: f.slug,
      fileCount: f.files.length,
      r: radiusFor(f.files.length),
      x: cx + Math.cos(theta) * ring,
      y: cy + Math.sin(theta) * ring,
      vx: 0,
      vy: 0,
      fx: null,
      fy: null,
    };
  });

  const fileSets = features.map((f) => new Set(f.files.map((x) => x.path)));
  const pairWeight = new Map<string, number>();
  const bump = (i: number, j: number, w: number) => {
    if (i === j) return;
    const key = i < j ? `${i}-${j}` : `${j}-${i}`;
    pairWeight.set(key, (pairWeight.get(key) ?? 0) + w);
  };

  // Shared files → weight = shared-file count.
  const fileToFeatures = new Map<string, number[]>();
  features.forEach((f, i) => {
    for (const file of fileSets[i]) {
      const arr = fileToFeatures.get(file);
      if (arr) arr.push(i);
      else fileToFeatures.set(file, [i]);
    }
  });
  for (const owners of fileToFeatures.values()) {
    for (let a = 0; a < owners.length; a++) {
      for (let b = a + 1; b < owners.length; b++) bump(owners[a], owners[b], 1);
    }
  }

  // Entry-point linkage: feature i's entry point lives in feature j's files.
  features.forEach((f, i) => {
    for (const ep of f.entryPoints) {
      for (let j = 0; j < features.length; j++) {
        if (j !== i && fileSets[j].has(ep)) bump(i, j, 1);
      }
    }
  });

  const edges: GEdge[] = [];
  for (const [key, weight] of pairWeight) {
    const [a, b] = key.split("-").map(Number);
    edges.push({ a, b, weight });
  }
  return { nodes, edges };
}

const REPULSION = 5200;
const SPRING = 0.02;
const GRAVITY = 0.02;
const DAMPING = 0.82;

function restLength(weight: number): number {
  return Math.max(58, 150 - weight * 16);
}

/** One integration tick. `alpha` cools the whole step; returns kinetic energy so
 *  the caller can stop the loop once the layout settles. */
export function step(g: Graph, bounds: SimBounds, alpha: number): number {
  const cx = bounds.width / 2;
  const cy = bounds.height / 2;
  const nodes = g.nodes;

  for (const nd of nodes) {
    nd.vx += (cx - nd.x) * GRAVITY * alpha;
    nd.vy += (cy - nd.y) * GRAVITY * alpha;
  }

  for (let i = 0; i < nodes.length; i++) {
    const a = nodes[i];
    for (let j = i + 1; j < nodes.length; j++) {
      const b = nodes[j];
      let dx = a.x - b.x;
      let dy = a.y - b.y;
      let d2 = dx * dx + dy * dy;
      if (d2 < 0.01) {
        dx = (Math.random() - 0.5) * 0.1;
        dy = (Math.random() - 0.5) * 0.1;
        d2 = dx * dx + dy * dy + 0.01;
      }
      const dist = Math.sqrt(d2);
      const force = (REPULSION / d2) * alpha;
      const fx = (dx / dist) * force;
      const fy = (dy / dist) * force;
      a.vx += fx;
      a.vy += fy;
      b.vx -= fx;
      b.vy -= fy;
    }
  }

  for (const e of g.edges) {
    const a = nodes[e.a];
    const b = nodes[e.b];
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const dist = Math.sqrt(dx * dx + dy * dy) || 0.01;
    const rest = restLength(e.weight);
    const f = SPRING * (dist - rest) * alpha;
    const fx = (dx / dist) * f;
    const fy = (dy / dist) * f;
    a.vx += fx;
    a.vy += fy;
    b.vx -= fx;
    b.vy -= fy;
  }

  let energy = 0;
  for (const nd of nodes) {
    if (nd.fx !== null && nd.fy !== null) {
      nd.x = nd.fx;
      nd.y = nd.fy;
      nd.vx = 0;
      nd.vy = 0;
      continue;
    }
    nd.vx *= DAMPING;
    nd.vy *= DAMPING;
    nd.x += nd.vx;
    nd.y += nd.vy;
    nd.x = Math.max(nd.r + 4, Math.min(bounds.width - nd.r - 4, nd.x));
    nd.y = Math.max(nd.r + 4, Math.min(bounds.height - nd.r - 4, nd.y));
    energy += nd.vx * nd.vx + nd.vy * nd.vy;
  }
  return energy;
}
