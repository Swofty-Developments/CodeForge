/* Stable per-slug hue from the Zed accent set, and the swatch palette offered by
 * the node color picker. A feature's explicit `color` always wins; otherwise a
 * deterministic hash of the slug keeps a feature the same color across sessions. */

import type { Feature } from "../../types";

/** The Zed One Dark accent set (see styles/global.css). */
export const GRAPH_PALETTE = [
  "#74ade8", // sky/primary
  "#a1c181", // green
  "#dec184", // amber
  "#b478c0", // purple
  "#cd7ca8", // pink
  "#d3a86a", // orange
  "#d07277", // red
  "#6eb4bf", // teal
] as const;

function hashSlug(slug: string): number {
  let h = 0;
  for (let i = 0; i < slug.length; i++) h = (h * 31 + slug.charCodeAt(i)) | 0;
  return Math.abs(h);
}

/** The default (unset) hue for a slug — stable across runs. */
export function hueForSlug(slug: string): string {
  return GRAPH_PALETTE[hashSlug(slug) % GRAPH_PALETTE.length];
}

/** The color a node is drawn in: explicit user color, else the stable hue. */
export function nodeColor(feature: Pick<Feature, "slug" | "color">): string {
  return feature.color ?? hueForSlug(feature.slug);
}
