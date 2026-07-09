/* Permission-mode switcher (capability C) — a small Zed dropdown near the
 * composer. Reflects the active session's live mode (or the pending mode for the
 * next session) and switches it. "Auto" (bypassPermissions) carries an amber
 * warning affordance since it auto-runs every tool without asking. */

import { For, Show, createSignal } from "solid-js";
import type { PermissionMode } from "../../types";

interface ModeDef {
  value: PermissionMode;
  label: string;
  desc: string;
}

export const MODES: ModeDef[] = [
  { value: "default", label: "Ask", desc: "Approve each tool" },
  { value: "acceptEdits", label: "Accept edits", desc: "Auto-accept file edits" },
  { value: "plan", label: "Plan", desc: "Plan only — no changes" },
  { value: "bypassPermissions", label: "Auto", desc: "Run everything, no prompts" },
];

export function modeLabel(mode: PermissionMode): string {
  return MODES.find((m) => m.value === mode)?.label ?? mode;
}

export function ModeControl(props: {
  mode: PermissionMode;
  disabled?: boolean;
  onSelect: (mode: PermissionMode) => void;
}) {
  const [open, setOpen] = createSignal(false);
  const isAuto = () => props.mode === "bypassPermissions";

  function pick(mode: PermissionMode): void {
    setOpen(false);
    if (mode !== props.mode) props.onSelect(mode);
  }

  return (
    <div class="mode-wrap">
      <button
        class="mode-btn"
        classList={{ "mode-btn--auto": isAuto(), "mode-btn--open": open() }}
        disabled={props.disabled}
        title={isAuto() ? "Auto: runs every tool without asking" : "Permission mode"}
        onClick={() => setOpen((o) => !o)}
      >
        <Show
          when={isAuto()}
          fallback={
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 3l7 4v5c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V7z" />
            </svg>
          }
        >
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0zM12 9v4M12 17h.01" />
          </svg>
        </Show>
        <span class="mode-label">{modeLabel(props.mode)}</span>
        <svg class="mode-caret" width="9" height="9" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M4 6l4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
        </svg>
      </button>

      <Show when={open()}>
        <div class="mode-backdrop" onClick={() => setOpen(false)} />
        <div class="mode-menu">
          <For each={MODES}>
            {(m) => (
              <button
                class="mode-item"
                classList={{ "mode-item--active": m.value === props.mode, "mode-item--auto": m.value === "bypassPermissions" }}
                onClick={() => pick(m.value)}
              >
                <span class="mode-item-label">{m.label}</span>
                <span class="mode-item-desc">{m.desc}</span>
              </button>
            )}
          </For>
        </div>
      </Show>

      <style>{`
        .mode-wrap { position: relative; display: flex; }
        .mode-btn {
          display: inline-flex; align-items: center; gap: 5px;
          height: 22px; padding: 0 7px;
          border-radius: var(--radius-pill);
          background: var(--bg-muted);
          border: 1px solid var(--border);
          color: var(--text-secondary);
          font-size: 10.5px; font-weight: 500;
          flex-shrink: 0;
        }
        .mode-btn:hover:not(:disabled) { background: var(--bg-accent); color: var(--text); }
        .mode-btn:disabled { opacity: 0.5; cursor: default; }
        .mode-btn--open { background: var(--bg-accent); color: var(--text); }
        .mode-btn--auto {
          background: rgba(var(--amber-rgb), 0.1);
          border-color: rgba(var(--amber-rgb), 0.4);
          color: var(--amber);
        }
        .mode-btn--auto:hover:not(:disabled) { background: rgba(var(--amber-rgb), 0.16); color: var(--amber); }
        .mode-label { line-height: 1; }
        .mode-caret { opacity: 0.6; flex-shrink: 0; }

        .mode-backdrop { position: fixed; inset: 0; z-index: 99; }
        .mode-menu {
          position: absolute;
          bottom: calc(100% + 6px); left: 0;
          z-index: 100;
          min-width: 180px;
          padding: 4px;
          background: var(--bg-elevated);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-md);
          box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
          animation: dropdown-in 0.14s var(--ease-out);
        }
        .mode-item {
          display: flex; flex-direction: column; gap: 1px;
          width: 100%; padding: 6px 8px; text-align: left;
          border-radius: var(--radius-sm);
        }
        .mode-item:hover { background: var(--bg-accent); }
        .mode-item-label { font-size: 12px; font-weight: 500; color: var(--text); }
        .mode-item--active .mode-item-label { color: var(--primary); }
        .mode-item--auto .mode-item-label { color: var(--amber); }
        .mode-item-desc { font-size: 10px; color: var(--text-tertiary); }
      `}</style>
    </div>
  );
}
