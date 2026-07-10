/* One immutable timeline row: 6px session lane dot, mono kind glyph,
 * kind-specific title, feature chips, relative timestamp, and an expandable
 * (grid 0fr→1fr) pretty-JSON payload. No edit/delete affordances by design. */

import { For, Show, createSignal } from "solid-js";
import type { TimelineEvent } from "../../types";
import {
  KIND_GLYPH,
  KIND_LABEL,
  docRefreshFailed,
  eventTitle,
  laneColor,
  prettyPayload,
  relativeTime,
  shortId,
} from "./timeline-utils";

// Rows render up to 500x — inject styles once instead of per-instance <style>.
if (!document.getElementById("tl-row-styles")) {
  const style = document.createElement("style");
  style.id = "tl-row-styles";
  style.textContent = `
    .tlr { border-radius: var(--radius-sm); }
    .tlr--live { animation: streaming-line-in 180ms var(--ease-out) both; }
    .tlr-head {
      display: flex; align-items: center; gap: var(--space-2);
      width: 100%; padding: 4px 10px;
      border-radius: var(--radius-sm);
      text-align: left; cursor: pointer;
    }
    .tlr-head:hover { background: var(--bg-hover); }
    .tlr-lane { width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; }
    .tlr-glyph {
      width: 14px; flex-shrink: 0; text-align: center;
      font-family: var(--font-mono); font-size: 11px;
      color: var(--text-tertiary);
    }
    .tlr-title {
      max-width: 55%; min-width: 0;
      font-size: 12px; color: var(--text-secondary);
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    .tlr-title--mono { font-family: var(--font-mono); font-size: 11.5px; }
    .tlr-title--failed { color: var(--red); }
    .tlr-chips { flex: 1; display: flex; gap: 4px; overflow: hidden; min-width: 0; }
    .tlr-chip {
      font-size: 9px; font-family: var(--font-mono); line-height: 1.6;
      padding: 0 6px; border-radius: var(--radius-pill);
      color: var(--primary); background: rgba(var(--primary-rgb), 0.08);
      white-space: nowrap; flex-shrink: 0;
    }
    .tlr-chevron {
      flex-shrink: 0; color: var(--text-tertiary); opacity: 0;
      transition: transform 0.18s ease;
    }
    .tlr-head:hover .tlr-chevron { opacity: 0.7; }
    .tlr-chevron--open { transform: rotate(90deg); opacity: 0.7; }
    .tlr-ts {
      flex-shrink: 0; min-width: 32px; text-align: right;
      font-size: 10px; font-family: var(--font-mono);
      color: var(--text-tertiary); font-variant-numeric: tabular-nums;
    }
    .tlr-body { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.2s ease; }
    .tlr-body--open { grid-template-rows: 1fr; }
    .tlr-body-inner { overflow: hidden; min-height: 0; }
    .tlr-meta {
      display: flex; gap: 10px; padding: 6px 10px 2px 30px;
      font-size: 10px; font-family: var(--font-mono); color: var(--text-tertiary);
    }
    .tlr-json {
      margin: 4px 10px 8px 30px; padding: 8px 10px;
      background: var(--bg-base); border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      font-family: var(--font-mono); font-size: 11px; line-height: 1.55;
      color: var(--text-secondary);
      max-height: 240px; overflow: auto;
      white-space: pre-wrap; word-break: break-word;
      user-select: text; -webkit-user-select: text;
    }
  `;
  document.head.appendChild(style);
}

export function TimelineEventRow(props: { event: TimelineEvent; now: number; live: boolean }) {
  const [open, setOpen] = createSignal(false);
  // Latch so the payload <pre> only enters the DOM after first expand.
  const [opened, setOpened] = createSignal(false);

  function toggle() {
    setOpen(!open());
    if (open()) setOpened(true);
  }

  const isMono = () => props.event.kind === "file_edited" || props.event.kind === "command_run";

  return (
    <div class="tlr" classList={{ "tlr--live": props.live }}>
      <button class="tlr-head" aria-expanded={open()} onClick={toggle}>
        <span class="tlr-lane" style={{ background: laneColor(props.event.sessionId) }} />
        <span class="tlr-glyph" title={KIND_LABEL[props.event.kind]}>
          {KIND_GLYPH[props.event.kind]}
        </span>
        <span
          class="tlr-title"
          classList={{
            "tlr-title--mono": isMono(),
            "tlr-title--failed": docRefreshFailed(props.event),
          }}
        >
          {eventTitle(props.event)}
        </span>
        <span class="tlr-chips">
          <For each={props.event.featureSlugs}>{(slug) => <span class="tlr-chip">{slug}</span>}</For>
        </span>
        <svg
          class="tlr-chevron"
          classList={{ "tlr-chevron--open": open() }}
          width="10"
          height="10"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M9 18l6-6-6-6" />
        </svg>
        <span class="tlr-ts">{relativeTime(props.event.ts, props.now)}</span>
      </button>
      <div class="tlr-body" classList={{ "tlr-body--open": open() }}>
        <div class="tlr-body-inner">
          <Show when={opened()}>
            <div class="tlr-meta">
              <span>{props.event.actor}</span>
              <span>{KIND_LABEL[props.event.kind]}</span>
              <Show when={props.event.sessionId}>
                <span title={props.event.sessionId ?? undefined}>
                  session {shortId(props.event.sessionId)}
                </span>
              </Show>
              <span>{new Date(props.event.ts).toLocaleString()}</span>
            </div>
            <pre class="tlr-json">{prettyPayload(props.event.payload)}</pre>
          </Show>
        </div>
      </div>
    </div>
  );
}
