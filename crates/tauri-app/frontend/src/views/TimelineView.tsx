/* Timeline view — reverse-chron immutable event list, live via timeline:event.
 * Placeholder empty state until the backend is wired. */

import { For, Show } from "solid-js";
import type { EventKind } from "../types";
import { appStore } from "../stores/app-store";

const KIND_LABEL: Record<EventKind, string> = {
  session_started: "Session started",
  session_ended: "Session ended",
  file_edited: "File edited",
  command_run: "Command run",
  tests_run: "Tests run",
  index_started: "Index started",
  index_completed: "Index completed",
  feature_pinned: "Feature pinned",
  feature_edited: "Feature edited",
  note: "Note",
};

export function TimelineView() {
  const { store } = appStore;

  return (
    <div class="tl">
      <Show
        when={store.timeline.length > 0}
        fallback={
          <div class="tl-empty">
            <div class="tl-empty-title">Nothing on the timeline yet</div>
            <div class="tl-empty-sub">
              Once hooks are installed, every edit, command and session lands here —
              classified by feature, live.
            </div>
          </div>
        }
      >
        <div class="tl-list">
          <For each={store.timeline}>
            {(event) => (
              <div class="tl-row">
                <span
                  class="tl-lane"
                  classList={{
                    "tl-lane--agent": event.actor === "agent",
                    "tl-lane--human": event.actor === "human",
                  }}
                />
                <span class="tl-kind">{KIND_LABEL[event.kind]}</span>
                <span class="tl-slugs">
                  <For each={event.featureSlugs}>
                    {(slug) => <span class="tl-slug">{slug}</span>}
                  </For>
                </span>
                <span class="tl-ts">{new Date(event.ts).toLocaleTimeString()}</span>
              </div>
            )}
          </For>
        </div>
      </Show>

      <style>{`
        .tl { flex: 1; display: flex; flex-direction: column; min-height: 0; }
        .tl-empty { margin: auto; text-align: center; max-width: 340px; animation: fade-slide-up 0.22s var(--ease-out) both; }
        .tl-empty-title { font-size: 15px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
        .tl-empty-sub { font-size: 12px; line-height: 1.5; color: var(--text-tertiary); }
        .tl-list {
          max-width: 768px; width: 100%; margin: 0 auto;
          padding: var(--space-4);
          display: flex; flex-direction: column; gap: 2px;
        }
        .tl-row {
          display: flex; align-items: center; gap: var(--space-3);
          padding: 6px 10px;
          border-radius: var(--radius-sm);
          animation: fade-slide-up 0.15s var(--ease-out) both;
        }
        .tl-row:hover { background: var(--bg-hover); }
        .tl-lane { width: 2px; height: 16px; border-radius: 1px; background: var(--text-tertiary); flex-shrink: 0; }
        .tl-lane--agent { background: var(--primary); }
        .tl-lane--human { background: var(--green); }
        .tl-kind { font-size: 12px; font-weight: 500; color: var(--text-secondary); white-space: nowrap; }
        .tl-slugs { flex: 1; display: flex; gap: 4px; overflow: hidden; }
        .tl-slug {
          font-size: 9px; font-family: var(--font-mono);
          padding: 0 6px; border-radius: var(--radius-pill);
          color: var(--primary); background: rgba(var(--primary-rgb), 0.08);
          white-space: nowrap;
        }
        .tl-ts { font-size: 10px; font-family: var(--font-mono); color: var(--text-tertiary); font-variant-numeric: tabular-nums; }
      `}</style>
    </div>
  );
}
