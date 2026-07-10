/* Merges the live store timeline (prepended by timeline:event) with locally
 * paged-in older events and filter refetches, deduped by append-only rowid. */

import { createMemo, createSignal } from "solid-js";
import * as ipc from "../../ipc";
import { appStore } from "../../stores/app-store";
import type { TimelineEvent, TimelineFilter } from "../../types";

const PAGE_SIZE = 200;

export function useTimelineEvents() {
  const { store } = appStore;
  const [extra, setExtra] = createSignal<TimelineEvent[]>([]);
  const [loadingOlder, setLoadingOlder] = createSignal(false);
  const [exhausted, setExhausted] = createSignal(false);

  /** Reverse-chron by id (rowid is append-only, so id order == ts order). */
  const events = createMemo<TimelineEvent[]>(() => {
    const seen = new Set<number>();
    const merged: TimelineEvent[] = [];
    for (const e of store.timeline) {
      if (!seen.has(e.id)) {
        seen.add(e.id);
        merged.push(e);
      }
    }
    for (const e of extra()) {
      if (!seen.has(e.id)) {
        seen.add(e.id);
        merged.push(e);
      }
    }
    merged.sort((a, b) => b.id - a.id);
    return merged;
  });

  function mergeIn(page: TimelineEvent[]): number {
    const known = new Set(events().map((e) => e.id));
    const fresh = page.filter((e) => !known.has(e.id));
    if (fresh.length > 0) setExtra((x) => [...x, ...fresh]);
    return fresh.length;
  }

  /** Page older events in via the strictly-below id cursor. A short page is a
   *  definitive answer: the store had nothing further back. */
  async function loadOlder(base: TimelineFilter): Promise<void> {
    const repo = store.repo;
    const oldest = events()[events().length - 1];
    if (!repo || !oldest || loadingOlder()) return;
    setLoadingOlder(true);
    try {
      const page = await ipc.getTimeline(repo.path, {
        ...base,
        beforeId: oldest.id,
        limit: PAGE_SIZE,
      });
      mergeIn(page);
      if (page.length < PAGE_SIZE) setExhausted(true);
    } catch (e) {
      console.error("timeline: load older failed", e);
    } finally {
      setLoadingOlder(false);
    }
  }

  /** Backend refetch on mount and when the filter narrows. The result is
   *  newest-first and limited, so a short page also settles exhaustion — the
   *  "Load older" affordance only appears when older events truly exist. */
  async function refetch(filter: TimelineFilter): Promise<void> {
    const repo = store.repo;
    if (!repo) return;
    setExhausted(false);
    try {
      const page = await ipc.getTimeline(repo.path, { ...filter, limit: PAGE_SIZE });
      mergeIn(page);
      if (page.length < PAGE_SIZE) setExhausted(true);
    } catch (e) {
      console.error("timeline: filtered refetch failed", e);
    }
  }

  return { events, loadOlder, refetch, loadingOlder, exhausted };
}
