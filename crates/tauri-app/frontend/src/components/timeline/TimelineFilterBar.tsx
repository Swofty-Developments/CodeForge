/* Timeline filter bar: single-select feature chips, single-select actor toggle,
 * multi-select kind chips, plus a clear button when anything is narrowed. */

import { For, Show } from "solid-js";
import type { Actor, EventKind } from "../../types";
import {
  ALL_ACTORS,
  ALL_KINDS,
  KIND_GLYPH,
  KIND_SHORT,
  filtersNarrowed,
  type TimelineFilters,
} from "./timeline-utils";

export interface FeatureChip {
  slug: string;
  name: string;
}

export function TimelineFilterBar(props: {
  features: FeatureChip[];
  filters: TimelineFilters;
  onFeature: (slug: string | null) => void;
  onActor: (actor: Actor | null) => void;
  onKind: (kind: EventKind) => void;
  onClear: () => void;
}) {
  return (
    <div class="tlf">
      <Show when={props.features.length > 0}>
        <div class="tlf-cluster">
          <span class="tlf-label">Feature</span>
          <For each={props.features}>
            {(f) => (
              <button
                class="tlf-chip"
                classList={{ "tlf-chip--on": props.filters.feature === f.slug }}
                onClick={() => props.onFeature(props.filters.feature === f.slug ? null : f.slug)}
              >
                {f.name}
              </button>
            )}
          </For>
        </div>
        <span class="tlf-divider" />
      </Show>

      <div class="tlf-cluster">
        <span class="tlf-label">Actor</span>
        <For each={ALL_ACTORS}>
          {(actor) => (
            <button
              class="tlf-chip"
              classList={{ "tlf-chip--on": props.filters.actor === actor }}
              onClick={() => props.onActor(props.filters.actor === actor ? null : actor)}
            >
              {actor}
            </button>
          )}
        </For>
      </div>
      <span class="tlf-divider" />

      <div class="tlf-cluster">
        <span class="tlf-label">Kind</span>
        <For each={ALL_KINDS}>
          {(kind) => (
            <button
              class="tlf-chip"
              classList={{ "tlf-chip--on": props.filters.kinds.includes(kind) }}
              onClick={() => props.onKind(kind)}
            >
              <span class="tlf-glyph">{KIND_GLYPH[kind]}</span>
              {KIND_SHORT[kind]}
            </button>
          )}
        </For>
      </div>

      <Show when={filtersNarrowed(props.filters)}>
        <button class="tlf-clear" onClick={props.onClear}>
          clear
        </button>
      </Show>

      <style>{`
        .tlf {
          display: flex; flex-wrap: wrap; align-items: center;
          gap: var(--space-1) var(--space-2);
          padding: var(--space-2) var(--space-4);
        }
        .tlf-cluster { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-1); }
        .tlf-label {
          font-size: 9px; font-weight: 600; text-transform: uppercase;
          letter-spacing: 0.08em; color: var(--text-tertiary);
          margin-right: 2px;
        }
        .tlf-divider { width: 1px; height: 14px; background: var(--border); flex-shrink: 0; }
        .tlf-chip {
          display: inline-flex; align-items: center; gap: 5px;
          font-size: 11px; font-weight: 500; color: var(--text-secondary);
          padding: 2px 9px; border-radius: var(--radius-pill);
          background: var(--bg-muted); border: 1px solid var(--border);
          white-space: nowrap;
          transition: background 0.15s, border-color 0.15s, color 0.15s;
        }
        .tlf-chip:hover { background: var(--bg-accent); border-color: var(--border-strong); }
        .tlf-chip--on {
          background: rgba(var(--primary-rgb), 0.1);
          border-color: rgba(var(--primary-rgb), 0.3);
          color: var(--primary);
        }
        .tlf-glyph { font-family: var(--font-mono); font-size: 10px; opacity: 0.8; }
        .tlf-clear {
          font-size: 10px; font-family: var(--font-mono);
          color: var(--text-tertiary); padding: 2px 8px;
          border-radius: var(--radius-pill);
          transition: color 0.15s, background 0.15s;
        }
        .tlf-clear:hover { color: var(--red); background: rgba(var(--red-rgb), 0.08); }
      `}</style>
    </div>
  );
}
