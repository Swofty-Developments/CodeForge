/* Timeline view — reverse-chron immutable event list, live via timeline:event.
 * Composes the filter bar + event rows over the merged (live + paged) feed.
 * Client-side filtering with a backend refetch when narrowed; caps rendered
 * rows and pages older events via the beforeId cursor. Zero edit affordances. */

import { For, Show, createComputed, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { createStore } from "solid-js/store";
import type { Actor, EventKind } from "../types";
import { appStore } from "../stores/app-store";
import { TimelineEventRow } from "../components/timeline/TimelineEventRow";
import { TimelineFilterBar, type FeatureChip } from "../components/timeline/TimelineFilterBar";
import { useTimelineEvents } from "../components/timeline/use-timeline-events";
import {
  filtersNarrowed,
  matchesFilters,
  toIpcFilter,
  type TimelineFilters,
} from "../components/timeline/timeline-utils";

const RENDER_STEP = 500;

if (!document.getElementById("tlv-styles")) {
  const style = document.createElement("style");
  style.id = "tlv-styles";
  style.textContent = `
    .tlv { flex: 1; display: flex; flex-direction: column; min-height: 0; }
    .tlv-filter {
      position: sticky; top: 0; z-index: 2;
      background: var(--bg-base); border-bottom: 1px solid var(--border);
    }
    .tlv-list {
      max-width: 820px; width: 100%; margin: 0 auto;
      padding: var(--space-3) var(--space-4) var(--space-8);
      display: flex; flex-direction: column; gap: 1px;
    }
    .tlv-more {
      display: flex; justify-content: center; padding: var(--space-4) 0 0;
    }
    .tlv-more-btn {
      font-size: 11px; font-weight: 500; color: var(--text-secondary);
      padding: 5px 16px; border-radius: var(--radius-pill);
      background: var(--bg-muted); border: 1px solid var(--border);
    }
    .tlv-more-btn:hover:not(:disabled) { background: var(--bg-accent); border-color: var(--border-strong); color: var(--text); }
    .tlv-more-btn:disabled { opacity: 0.5; cursor: default; }
    .tlv-nomatch {
      margin: auto; text-align: center; padding: var(--space-8) var(--space-4);
      display: flex; flex-direction: column; align-items: center; gap: 8px;
    }
    .tlv-nomatch-text { font-size: 12px; color: var(--text-tertiary); }
    .tlv-nomatch-clear {
      font-size: 11px; font-family: var(--font-mono); color: var(--primary);
      padding: 2px 10px; border-radius: var(--radius-pill);
      background: rgba(var(--primary-rgb), 0.08); border: 1px solid rgba(var(--primary-rgb), 0.2);
    }
    .tlv-empty { margin: auto; text-align: center; max-width: 360px; animation: fade-slide-up 0.22s var(--ease-out) both; }
    .tlv-empty-title { font-size: 15px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
    .tlv-empty-sub { font-size: 12px; line-height: 1.5; color: var(--text-tertiary); }
  `;
  document.head.appendChild(style);
}

export function TimelineView() {
  const { store } = appStore;
  const { events, loadOlder, refetch, loadingOlder, exhausted } = useTimelineEvents();

  const [filters, setFilters] = createStore<TimelineFilters>({ feature: null, actor: null, kinds: [] });
  const [renderLimit, setRenderLimit] = createSignal(RENDER_STEP);
  const [now, setNow] = createSignal(Date.now());

  const clock = setInterval(() => setNow(Date.now()), 30_000);
  onCleanup(() => clearInterval(clock));

  // Settle exhaustion up front: one probe fetch on mount tells us whether older
  // events exist at all, so "Load older" never shows on a fully-loaded feed.
  onMount(() => void refetch(toIpcFilter(snapshot())));

  // Baseline = highest event id present when the feed first loads. Anything with
  // a larger id arrived live afterward and gets the streaming-in animation.
  const [baseline, setBaseline] = createSignal<number | null>(null);
  createComputed(() => {
    const evs = events();
    if (baseline() === null && evs.length > 0) setBaseline(evs[0].id);
  });
  const isLive = (id: number) => baseline() !== null && id > baseline()!;

  const featureChips = createMemo<FeatureChip[]>(() =>
    store.features.map((f) => ({ slug: f.slug, name: f.name })),
  );

  const snapshot = (): TimelineFilters => ({
    feature: filters.feature,
    actor: filters.actor,
    kinds: [...filters.kinds],
  });

  // Every filter change re-probes: exhaustion is scoped to the fetched filter,
  // so widening (or clearing) must re-settle it for the new scope.
  function refetchScope(): void {
    void refetch(toIpcFilter(snapshot()));
  }

  function onFeature(slug: string | null): void {
    setFilters("feature", slug);
    refetchScope();
  }
  function onActor(actor: Actor | null): void {
    setFilters("actor", actor);
    refetchScope();
  }
  function onKind(kind: EventKind): void {
    setFilters("kinds", (ks) => (ks.includes(kind) ? ks.filter((k) => k !== kind) : [...ks, kind]));
    refetchScope();
  }
  function onClear(): void {
    setFilters({ feature: null, actor: null, kinds: [] });
    refetchScope();
  }

  const filtered = createMemo(() => {
    const f = snapshot();
    if (!filtersNarrowed(f)) return events();
    return events().filter((e) => matchesFilters(e, f));
  });

  const visible = createMemo(() => filtered().slice(0, renderLimit()));
  const hasMore = () => filtered().length > visible().length || !exhausted();

  async function onLoadOlder(): Promise<void> {
    if (filtered().length > renderLimit()) {
      setRenderLimit(renderLimit() + RENDER_STEP);
      return;
    }
    await loadOlder(toIpcFilter(snapshot()));
    setRenderLimit(renderLimit() + RENDER_STEP);
  }

  return (
    <div class="tlv">
      <Show
        when={events().length > 0}
        fallback={
          <div class="tlv-empty">
            <div class="tlv-empty-title">No timeline yet</div>
            <div class="tlv-empty-sub">Events will appear as agents work.</div>
          </div>
        }
      >
        <div class="tlv-filter">
          <TimelineFilterBar
            features={featureChips()}
            filters={filters}
            onFeature={onFeature}
            onActor={onActor}
            onKind={onKind}
            onClear={onClear}
          />
        </div>

        <Show
          when={visible().length > 0}
          fallback={
            <div class="tlv-nomatch">
              <span class="tlv-nomatch-text">No events match these filters.</span>
              <button class="tlv-nomatch-clear" onClick={onClear}>
                clear filters
              </button>
            </div>
          }
        >
          <div class="tlv-list">
            <For each={visible()}>
              {(event) => <TimelineEventRow event={event} now={now()} live={isLive(event.id)} />}
            </For>
            <Show when={hasMore()}>
              <div class="tlv-more">
                <button class="tlv-more-btn" disabled={loadingOlder()} onClick={() => void onLoadOlder()}>
                  {loadingOlder() ? "Loading…" : "Load older"}
                </button>
              </div>
            </Show>
          </div>
        </Show>
      </Show>
    </div>
  );
}
