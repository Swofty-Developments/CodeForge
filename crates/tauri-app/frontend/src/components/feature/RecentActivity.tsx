/* Recent-activity list for the selected feature — the last N timeline events,
 * newest first, each with a kind glyph, mono relative time and a one-line
 * summary. New live events slide in as they arrive. */

import { For, Show } from "solid-js";
import type { TimelineEvent } from "../../types";
import { formatAgo } from "../sidebar/time";
import { kindAccent, kindGlyph, summarize } from "./event-summary";

export function RecentActivity(props: { events: TimelineEvent[]; now: number }) {
  return (
    <div class="ra">
      <Show
        when={props.events.length > 0}
        fallback={<div class="ra-empty">No activity recorded for this feature yet.</div>}
      >
        <For each={props.events}>
          {(ev) => (
            <div class="ra-row" style={{ "--ra-accent": `var(--${kindAccent(ev.kind)})` }}>
              <span class="ra-icon">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d={kindGlyph(ev.kind)} />
                </svg>
              </span>
              <span class="ra-summary">{summarize(ev)}</span>
              <span class="ra-time">{formatAgo(ev.ts, props.now)}</span>
            </div>
          )}
        </For>
      </Show>

      <style>{`
        .ra { display: flex; flex-direction: column; }
        .ra-empty { font-size: 12px; color: var(--text-tertiary); padding: var(--space-1) 0; }
        .ra-row {
          display: flex; align-items: center; gap: var(--space-2);
          padding: 5px 6px;
          border-radius: var(--radius-sm);
          animation: fade-slide-down 0.2s var(--ease-out) both;
        }
        .ra-row:hover { background: var(--bg-hover); }
        .ra-icon {
          display: flex; align-items: center; justify-content: center;
          width: 18px; height: 18px; border-radius: var(--radius-sm);
          flex-shrink: 0;
          color: var(--ra-accent);
          background: color-mix(in srgb, var(--ra-accent) 12%, transparent);
        }
        .ra-summary {
          flex: 1; min-width: 0;
          font-size: 12px; color: var(--text-secondary);
          overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
        }
        .ra-time {
          flex-shrink: 0;
          font-size: 10px; font-family: var(--font-mono); color: var(--text-tertiary);
          font-variant-numeric: tabular-nums;
        }
        @media (prefers-reduced-motion: reduce) {
          .ra-row { animation: none; }
        }
      `}</style>
    </div>
  );
}
