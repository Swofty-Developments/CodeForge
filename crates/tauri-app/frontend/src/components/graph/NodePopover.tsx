/* Side popover for the selected graph node — name, tags, file/entry counts, a
 * color-swatch picker (setFeatureColor), and a hand-off into the full feature
 * detail. Colors come from the Zed accent set; "Auto" clears the override. */

import { For, Show } from "solid-js";
import type { Feature } from "../../types";
import { appStore } from "../../stores/app-store";
import { GRAPH_PALETTE, nodeColor } from "./colors";

export function NodePopover(props: {
  feature: Feature;
  onOpenDetail: () => void;
  onClose: () => void;
}) {
  const f = () => props.feature;
  const current = () => nodeColor(f());

  return (
    <div class="gp-pop">
      <div class="gp-pop-head">
        <span class="gp-swatch" style={{ background: current() }} />
        <span class="gp-pop-name" title={f().name}>{f().name}</span>
        <button class="gp-pop-x" title="Close" onClick={() => props.onClose()}>
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <Show when={f().tags.length > 0}>
        <div class="gp-tags">
          <For each={f().tags}>{(t) => <span class="gp-tag">{t}</span>}</For>
        </div>
      </Show>

      <div class="gp-stats">
        <span><b>{f().files.length}</b> files</span>
        <span><b>{f().entryPoints.length}</b> entries</span>
        <span><b>{Math.round(f().confidence * 100)}%</b> conf</span>
      </div>

      <div class="gp-color-label">Node color</div>
      <div class="gp-swatches">
        <For each={GRAPH_PALETTE}>
          {(hex) => (
            <button
              class="gp-color"
              classList={{ "gp-color--on": f().color === hex }}
              style={{ background: hex }}
              title={hex}
              onClick={() => void appStore.setFeatureColor(f().slug, hex)}
            />
          )}
        </For>
        <button
          class="gp-color gp-color--clear"
          classList={{ "gp-color--on": !f().color }}
          title="Auto (default hue)"
          onClick={() => void appStore.setFeatureColor(f().slug, undefined)}
        >
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 3l18 18M21 3L3 21" />
          </svg>
        </button>
      </div>

      <button class="gp-open" onClick={() => props.onOpenDetail()}>
        Open feature detail
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M14 4h6v6M20 4l-9 9M18 13v6H5V6h6" />
        </svg>
      </button>

      <style>{`
        .gp-pop {
          position: absolute; top: 12px; right: 12px; z-index: 5;
          width: 236px;
          padding: 12px;
          background: var(--bg-elevated);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-lg);
          box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
          animation: dropdown-in 0.16s var(--ease-out);
        }
        .gp-pop-head { display: flex; align-items: center; gap: 8px; }
        .gp-swatch { width: 11px; height: 11px; border-radius: 3px; flex-shrink: 0; }
        .gp-pop-name {
          flex: 1; min-width: 0; font-size: 13px; font-weight: 600; color: var(--text);
          overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
        }
        .gp-pop-x {
          display: flex; align-items: center; justify-content: center;
          width: 18px; height: 18px; border-radius: var(--radius-sm);
          color: var(--text-tertiary); flex-shrink: 0;
        }
        .gp-pop-x:hover { background: var(--bg-accent); color: var(--text); }
        .gp-tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 9px; }
        .gp-tag {
          font-size: 9px; font-weight: 500; font-family: var(--font-mono);
          padding: 1px 6px; border-radius: var(--radius-pill);
          color: var(--purple); background: rgba(var(--purple-rgb), 0.1);
        }
        .gp-stats {
          display: flex; gap: 12px; margin-top: 10px;
          font-size: 10.5px; color: var(--text-tertiary);
        }
        .gp-stats b { color: var(--text-secondary); font-weight: 600; font-variant-numeric: tabular-nums; }
        .gp-color-label {
          font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em;
          color: var(--text-tertiary); margin: 12px 0 6px;
        }
        .gp-swatches { display: flex; flex-wrap: wrap; gap: 6px; }
        .gp-color {
          width: 20px; height: 20px; border-radius: var(--radius-sm);
          border: 1px solid rgba(255, 255, 255, 0.12);
          display: flex; align-items: center; justify-content: center;
        }
        .gp-color:hover { transform: scale(1.1); }
        .gp-color--on { box-shadow: 0 0 0 2px var(--bg-elevated), 0 0 0 3px var(--text); }
        .gp-color--clear { background: var(--bg-muted); color: var(--text-tertiary); }
        .gp-open {
          display: flex; align-items: center; justify-content: center; gap: 6px;
          width: 100%; margin-top: 14px; padding: 7px;
          font-size: 12px; font-weight: 600;
          color: var(--text-secondary);
          border: 1px solid var(--border); border-radius: var(--radius-md);
        }
        .gp-open:hover { background: var(--bg-accent); color: var(--text); }
        @media (prefers-reduced-motion: reduce) { .gp-color:hover { transform: none; } }
      `}</style>
    </div>
  );
}
