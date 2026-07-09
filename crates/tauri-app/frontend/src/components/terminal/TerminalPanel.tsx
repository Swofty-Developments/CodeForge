/* Bottom terminal panel (FZ-3): a resizable, collapsible dock with a tab strip
 * (one tab per PTY + a "+") over stacked xterm instances. Holds the id→Terminal
 * map and the terminal:data / terminal:exit listeners, routing output to the
 * matching instance. Styled to the Zed chrome — 1px hairline top border, tab strip
 * mirroring the worktree tabs. */

import { For, Show, onCleanup, onMount } from "solid-js";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import { TerminalInstance } from "./TerminalInstance";

/** Decode base64 PTY output to bytes so xterm renders UTF-8 correctly. */
function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

function basename(p: string): string {
  const parts = p.replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || p;
}

export function TerminalPanel() {
  const { store } = appStore;
  const termMap = new Map<string, Terminal>();
  const register = (id: string, term: Terminal): void => void termMap.set(id, term);
  const unregister = (id: string): void => void termMap.delete(id);

  let unData: UnlistenFn | undefined;
  let unExit: UnlistenFn | undefined;

  onMount(async () => {
    unData = await ipc.listenTerminalData((id, data) => {
      termMap.get(id)?.write(b64ToBytes(data));
    });
    unExit = await ipc.listenTerminalExit((id, code) => {
      appStore.markTerminalExited(id);
      const suffix = code != null ? ` — code ${code}` : "";
      termMap.get(id)?.write(`\r\n\x1b[38;5;244m[process exited${suffix}]\x1b[0m\r\n`);
    });
  });
  onCleanup(() => { unData?.(); unExit?.(); });

  // ── Drag the top edge to resize the panel height ──
  let dragging = false;
  let startY = 0;
  let startH = 0;
  function onMove(e: MouseEvent): void {
    if (dragging) appStore.setTerminalPanelHeight(startH + (startY - e.clientY));
  }
  function onUp(): void {
    if (!dragging) return;
    dragging = false;
    document.body.style.cursor = "";
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  }
  function onDragStart(e: MouseEvent): void {
    e.preventDefault();
    dragging = true;
    startY = e.clientY;
    startH = store.terminalPanelHeight;
    document.body.style.cursor = "row-resize";
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }
  onCleanup(() => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  });

  return (
    <div class="term-panel" style={{ height: `${store.terminalPanelHeight}px` }}>
      <div class="term-resize" onMouseDown={onDragStart} />
      <div class="term-tabs">
        <For each={store.terminals}>
          {(t) => (
            <div
              class="term-tab"
              classList={{ "term-tab--active": store.activeTerminalId === t.id, "term-tab--exited": t.exited }}
              title={t.cwd}
              onClick={() => appStore.setActiveTerminal(t.id)}
            >
              <span class="term-tab-dot" />
              <span class="term-tab-label">{t.title}</span>
              <Show when={t.exited}><span class="term-tab-tag">exited</span></Show>
              <button
                class="term-tab-close"
                title="Close terminal"
                onClick={(e) => { e.stopPropagation(); void appStore.closeTerminal(t.id); }}
              >
                <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.8">
                  <path d="M18 6L6 18M6 6l12 12" />
                </svg>
              </button>
            </div>
          )}
        </For>
        <button class="term-add" title="New terminal" onClick={() => void appStore.openTerminal()}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M12 5v14M5 12h14" />
          </svg>
        </button>
        <div class="term-flex" />
        <button class="term-collapse" title="Hide terminal (⌘J)" onClick={() => appStore.toggleTerminalPanel()}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>
      </div>
      <div class="term-body">
        <Show
          when={store.terminals.length > 0}
          fallback={
            <div class="term-empty">
              No terminals open.{" "}
              <button class="term-empty-btn" onClick={() => void appStore.openTerminal()}>Open one</button>{" "}
              <Show when={store.activeContextPath}>
                in <span class="term-empty-path">{basename(store.activeContextPath!)}</span>
              </Show>
            </div>
          }
        >
          <For each={store.terminals}>
            {(t) => (
              <TerminalInstance
                tab={t}
                active={store.activeTerminalId === t.id}
                register={register}
                unregister={unregister}
              />
            )}
          </For>
        </Show>
      </div>

      <style>{`
        .term-panel {
          position: relative;
          display: flex;
          flex-direction: column;
          flex-shrink: 0;
          min-height: 0;
          background: var(--bg-base);
          border-top: 1px solid var(--border);
        }
        /* Drag strip on the top edge — hairline that glows on hover, like the
           workspace column handles. */
        .term-resize {
          position: absolute;
          top: -3px; left: 0; right: 0;
          height: 6px;
          cursor: row-resize;
          z-index: 2;
        }
        .term-resize::after {
          content: "";
          position: absolute;
          left: 0; right: 0; top: 3px;
          height: 1px;
          background: transparent;
          transition: background 0.15s, box-shadow 0.15s;
        }
        .term-resize:hover::after { background: var(--primary); box-shadow: 0 0 6px var(--primary-glow); }

        .term-tabs {
          display: flex;
          align-items: center;
          gap: 3px;
          height: 30px;
          padding: 0 6px;
          background: var(--bg-surface);
          border-bottom: 1px solid var(--border-variant);
          flex-shrink: 0;
          overflow-x: auto;
          scrollbar-width: none;
        }
        .term-tabs::-webkit-scrollbar { display: none; }
        .term-tab {
          display: flex; align-items: center; gap: 6px;
          padding: 0 6px 0 9px;
          height: 22px;
          font-size: 11px;
          color: var(--text-tertiary);
          border: 1px solid transparent;
          border-radius: var(--radius-sm);
          white-space: nowrap;
          cursor: pointer;
          flex-shrink: 0;
        }
        .term-tab:hover { background: var(--bg-hover); color: var(--text-secondary); }
        .term-tab--active {
          color: var(--text);
          background: var(--bg-base);
          border-color: var(--border);
        }
        .term-tab-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--green); flex-shrink: 0; }
        .term-tab--exited .term-tab-dot { background: var(--text-tertiary); }
        .term-tab-label { font-family: var(--font-mono); font-size: 11px; max-width: 140px; overflow: hidden; text-overflow: ellipsis; }
        .term-tab-tag {
          font-size: 8.5px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em;
          color: var(--text-tertiary); background: var(--bg-muted);
          border: 1px solid var(--border-variant); padding: 0 4px; border-radius: var(--radius-pill);
        }
        .term-tab-close {
          display: flex; align-items: center; justify-content: center;
          width: 14px; height: 14px; border-radius: var(--radius-sm);
          color: var(--text-tertiary); opacity: 0; flex-shrink: 0;
        }
        .term-tab:hover .term-tab-close, .term-tab--active .term-tab-close { opacity: 0.7; }
        .term-tab-close:hover { opacity: 1; background: var(--bg-accent); color: var(--text); }
        .term-add, .term-collapse {
          display: flex; align-items: center; justify-content: center;
          width: 22px; height: 22px; border-radius: var(--radius-sm);
          color: var(--text-tertiary); flex-shrink: 0;
        }
        .term-add:hover, .term-collapse:hover { background: var(--bg-accent); color: var(--text-secondary); }
        .term-flex { flex: 1; }

        .term-body { position: relative; flex: 1; min-height: 0; overflow: hidden; }
        .term-host {
          position: absolute; inset: 0;
          padding: 4px 6px 2px 8px;
          visibility: hidden;
        }
        .term-host--active { visibility: visible; }
        .term-host .xterm { height: 100%; }
        .term-host .xterm-viewport { background: transparent !important; }

        .term-empty {
          display: flex; align-items: center; gap: 4px;
          height: 100%; padding: 0 12px;
          font-size: 12px; color: var(--text-tertiary);
        }
        .term-empty-btn { color: var(--primary); font-size: 12px; font-weight: 600; }
        .term-empty-btn:hover { text-decoration: underline; }
        .term-empty-path { font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary); }
      `}</style>
    </div>
  );
}
