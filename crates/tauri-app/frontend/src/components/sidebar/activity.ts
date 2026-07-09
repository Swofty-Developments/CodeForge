/* Per-feature activity derived from the loaded timeline slice, plus the
 * "seen up to event id" watermark that drives the unseen-count badges. */

import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";
import type { Feature, TimelineEvent } from "../../types";

const [seenStore, setSeenStore] = createRoot(() => createStore<Record<string, number>>({}));

/** Highest event id already seen per feature slug (selection watermark). */
export const seen = seenStore;

export function markSeen(slug: string, eventId: number): void {
  setSeenStore(slug, eventId);
}

/** Newest event timestamp (ms) per feature slug. */
export function latestActivity(timeline: readonly TimelineEvent[]): Map<string, number> {
  const latest = new Map<string, number>();
  for (const ev of timeline) {
    const t = Date.parse(ev.ts);
    if (Number.isNaN(t)) continue;
    for (const slug of ev.featureSlugs) {
      const prev = latest.get(slug);
      if (prev === undefined || t > prev) latest.set(slug, t);
    }
  }
  return latest;
}

/** Events newer than each feature's watermark — the sidebar badge counts. */
export function unseenCounts(
  timeline: readonly TimelineEvent[],
  seenMap: Record<string, number>,
): Map<string, number> {
  const counts = new Map<string, number>();
  for (const ev of timeline) {
    for (const slug of ev.featureSlugs) {
      if (ev.id > (seenMap[slug] ?? -1)) counts.set(slug, (counts.get(slug) ?? 0) + 1);
    }
  }
  return counts;
}

/** Most recent activity first, then name; falls back to updatedAt. */
export function sortFeatures(
  features: readonly Feature[],
  lastActivity: Map<string, number>,
): Feature[] {
  const ts = (f: Feature): number => {
    const fromTimeline = lastActivity.get(f.slug);
    if (fromTimeline !== undefined) return fromTimeline;
    const t = Date.parse(f.updatedAt);
    return Number.isNaN(t) ? 0 : t;
  };
  return [...features].sort((a, b) => {
    const ta = ts(a);
    const tb = ts(b);
    if (tb !== ta) return tb - ta;
    return a.name.localeCompare(b.name);
  });
}
