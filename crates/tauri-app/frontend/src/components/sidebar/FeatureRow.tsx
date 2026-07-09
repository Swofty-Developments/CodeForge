/* One feature-tree row: name, unseen-activity badge (sky), pin toggle (amber
 * when pinned, hover-reveal otherwise), confidence underline for low-confidence
 * features. Entrance staggers via inline animation-delay. */

import { Show } from "solid-js";
import type { Feature } from "../../types";

export function FeatureRow(props: {
  feature: Feature;
  active: boolean;
  activity: number;
  index: number;
  onSelect: (slug: string) => void;
  onTogglePin: (slug: string, pinned: boolean) => void;
}) {
  const f = () => props.feature;
  const lowConfidence = () => f().confidence < 0.75;

  return (
    <button
      class="ft-row"
      classList={{ "ft-row--active": props.active }}
      style={{ "animation-delay": `${props.index * 20}ms` }}
      onClick={() => props.onSelect(f().slug)}
      title={f().name}
    >
      <span class="ft-name">{f().name}</span>

      <Show when={props.activity > 0}>
        <span class="ft-badge">{props.activity > 99 ? "99+" : props.activity}</span>
      </Show>

      <span
        class="ft-pin"
        classList={{ "ft-pin--on": f().pinned }}
        role="button"
        aria-label={f().pinned ? "Unpin feature" : "Pin feature"}
        title={f().pinned ? "Unpin" : "Pin"}
        onClick={(e) => {
          e.stopPropagation();
          props.onTogglePin(f().slug, !f().pinned);
        }}
      >
        <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
          <path d="M16 3v2l-1 1v5l3 3v2h-5v6l-1 1-1-1v-6H6v-2l3-3V6L8 5V3h8z" />
        </svg>
      </span>

      <Show when={lowConfidence()}>
        <span class="ft-confbar" style={{ width: `${Math.max(8, Math.round(f().confidence * 100))}%` }} />
      </Show>

      <style>{`
        .ft-row {
          display: flex; align-items: center; gap: var(--space-2);
          width: calc(100% - 8px);
          padding: 6px var(--space-3) 7px;
          margin: 1px var(--space-1);
          border-radius: var(--radius-sm);
          cursor: pointer;
          text-align: left;
          position: relative;
          animation: fade-slide-up 0.16s var(--ease-out) both;
        }
        .ft-row:hover { background: var(--bg-hover); }
        .ft-row--active {
          background: rgba(var(--primary-rgb), 0.08);
        }
        .ft-row--active::before {
          content: ""; position: absolute; left: 0; top: 6px; bottom: 6px;
          width: 2px; border-radius: 1px; background: var(--primary);
        }
        .ft-name {
          flex: 1; min-width: 0;
          font-size: 13px; color: var(--text-secondary);
          overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
        }
        .ft-row--active .ft-name { color: var(--text); font-weight: 500; }

        .ft-badge {
          font-size: 9px; font-weight: 500; font-family: var(--font-mono);
          padding: 0 var(--space-1); border-radius: 3px; line-height: 1.55;
          color: var(--sky); background: rgba(var(--sky-rgb), 0.12);
          font-variant-numeric: tabular-nums; flex-shrink: 0;
        }

        .ft-pin {
          display: flex; align-items: center; flex-shrink: 0;
          color: var(--text-tertiary);
          opacity: 0;
          border-radius: var(--radius-sm);
        }
        .ft-row:hover .ft-pin { opacity: 0.55; }
        .ft-pin:hover { opacity: 1; background: var(--bg-accent); color: var(--text-secondary); }
        .ft-pin--on, .ft-row:hover .ft-pin--on { opacity: 1; color: var(--amber); }
        .ft-pin--on:hover { color: var(--amber); }

        /* Confidence underline — 2px bar, width == confidence, low-confidence only */
        .ft-confbar {
          position: absolute; left: var(--space-3); bottom: 2px;
          height: 2px; border-radius: 1px;
          background: rgba(var(--amber-rgb), 0.55);
          pointer-events: none;
        }
      `}</style>
    </button>
  );
}
