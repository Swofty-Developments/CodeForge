/* App shell: sidebar (feature tree) · main panel (tab bar / view / status bar) ·
 * right session pane · Cmd+K palette. Layout per docs/ARCHITECTURE.md §Frontend. */

import { For, Match, Show, Switch, onCleanup, onMount } from "solid-js";
import type { ActiveView } from "./types";
import { CommandPalette } from "./components/CommandPalette";
import { MergeResultPanel } from "./components/MergeResultPanel";
import { SessionPane } from "./components/SessionPane";
import { Sidebar } from "./components/Sidebar";
import { StatusBar } from "./components/StatusBar";
import { TabBar } from "./components/TabBar";
import { TitleBar } from "./components/TitleBar";
import { WorktreeTabs } from "./components/WorktreeTabs";
import { DiffReview } from "./views/DiffReview";
import { FeatureDetail } from "./views/FeatureDetail";
import { GraphView } from "./views/GraphView";
import { TimelineView } from "./views/TimelineView";
import { Welcome } from "./views/Welcome";
import { appStore } from "./stores/app-store";

export default function App() {
  const { store } = appStore;

  let sidebarDragging = false;
  let sessionDragging = false;

  function onSidebarDragStart(e: MouseEvent) {
    e.preventDefault();
    sidebarDragging = true;
    document.body.style.cursor = "col-resize";
  }

  function onSessionDragStart(e: MouseEvent) {
    e.preventDefault();
    sessionDragging = true;
    document.body.style.cursor = "col-resize";
  }

  function onMouseMove(e: MouseEvent) {
    if (sidebarDragging) appStore.setSidebarWidth(e.clientX);
    // Session pane hugs the right edge, so its width grows as the handle moves left.
    else if (sessionDragging) appStore.setSessionPaneWidth(window.innerWidth - e.clientX);
  }

  function onMouseUp() {
    if (sidebarDragging || sessionDragging) {
      sidebarDragging = false;
      sessionDragging = false;
      document.body.style.cursor = "";
    }
  }

  const VIEW_KEYS: Record<string, ActiveView> = {
    "1": "feature",
    "2": "graph",
    "3": "timeline",
    "4": "diff",
  };

  function onKeyDown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      appStore.setPaletteOpen(!store.paletteOpen);
    } else if (mod && e.key === "\\") {
      e.preventDefault();
      appStore.toggleSessionPane();
    } else if (mod && VIEW_KEYS[e.key] && store.repo) {
      e.preventDefault();
      appStore.setActiveView(VIEW_KEYS[e.key]);
    } else if (e.key === "Escape") {
      // Priority close: palette → pending approval (deny) → session pane.
      if (store.paletteOpen) {
        appStore.setPaletteOpen(false);
        return;
      }
      const withApproval =
        store.sessions.find((s) => s.info.id === store.activeSessionId && s.pendingApproval) ??
        store.sessions.find((s) => s.pendingApproval);
      if (withApproval?.pendingApproval) {
        void appStore.approveRequest(withApproval.info.id, withApproval.pendingApproval.requestId, false);
        return;
      }
      if (store.sessionPaneOpen) appStore.toggleSessionPane();
    }
  }

  onMount(() => {
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    window.addEventListener("keydown", onKeyDown);
  });
  onCleanup(() => {
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
    window.removeEventListener("keydown", onKeyDown);
  });

  return (
    <>
      <TitleBar />
      <Show when={store.repo}>
        <WorktreeTabs />
      </Show>
      <div class="workspace">
        <Show when={store.repo} fallback={<div class="workspace-welcome"><Welcome /></div>}>
          <Sidebar />
          <div class="resize-handle" onMouseDown={onSidebarDragStart} />
          <div class="main-panel">
            <TabBar />
            <div class="main-panel-body">
              <div class="main-panel-view">
                {/* Keyed on the active context so switching cross-fades + slides. */}
                <Show when={store.activeContextPath} keyed>
                  <div class="ctx-swap">
                    <Switch fallback={<FeatureDetail />}>
                      <Match when={store.activeView === "feature"}>
                        <FeatureDetail />
                      </Match>
                      <Match when={store.activeView === "graph"}>
                        <GraphView />
                      </Match>
                      <Match when={store.activeView === "timeline"}>
                        <TimelineView />
                      </Match>
                      <Match when={store.activeView === "diff"}>
                        <DiffReview />
                      </Match>
                    </Switch>
                  </div>
                </Show>
              </div>
              <Show when={store.sessionPaneOpen}>
                <div class="resize-handle resize-handle--session" onMouseDown={onSessionDragStart} />
                <SessionPane />
              </Show>
            </div>
          </div>
        </Show>
      </div>
      <StatusBar />
      <MergeResultPanel />
      <Show when={store.paletteOpen}>
        <CommandPalette />
      </Show>

      <Show when={store.toasts.length > 0}>
        <div class="toast-stack">
          <For each={store.toasts}>
            {(toast) => (
              <div
                class="toast"
                classList={{ "toast--success": toast.kind === "success" }}
                role="alert"
                title="Dismiss"
                onClick={() => appStore.dismissToast(toast.id)}
              >
                <span class="toast-dot" />
                <span class="toast-msg">{toast.message}</span>
              </div>
            )}
          </For>
        </div>
      </Show>

      <style>{`
        .main-panel {
          flex: 1;
          display: flex;
          flex-direction: column;
          min-width: 0;
          background: var(--bg-base);
        }
        .main-panel-body {
          flex: 1;
          display: flex;
          flex-direction: row;
          min-height: 0;
          overflow: hidden;
          position: relative;
        }
        .main-panel-view {
          flex: 1;
          min-width: 0;
          min-height: 0;
          overflow-y: auto;
          display: flex;
          flex-direction: column;
        }
        /* Cross-fade + horizontal slide when the active repo context changes. */
        .ctx-swap {
          flex: 1;
          min-height: 0;
          display: flex;
          flex-direction: column;
          animation: ctx-swap-in 0.28s var(--ease-out) both;
        }
        .resize-handle {
          width: 5px;
          cursor: col-resize;
          background: transparent;
          flex-shrink: 0;
          position: relative;
        }
        .resize-handle--session::after { left: 2px; }
        .resize-handle::after {
          content: "";
          position: absolute;
          left: 2px; top: 0; bottom: 0;
          width: 1px;
          background: var(--border);
          transition: background 0.15s, box-shadow 0.15s;
        }
        .resize-handle:hover::after {
          background: var(--primary);
          box-shadow: 0 0 6px var(--primary-glow);
        }

        /* ── Error toasts — bottom-right stack, semantic red tint ── */
        .toast-stack {
          position: fixed;
          right: 16px;
          bottom: 34px;
          display: flex;
          flex-direction: column;
          gap: 8px;
          z-index: 150;
          max-width: 360px;
        }
        .toast {
          display: flex;
          align-items: flex-start;
          gap: 8px;
          padding: 10px 12px;
          background:
            linear-gradient(rgba(var(--red-rgb), 0.08), rgba(var(--red-rgb), 0.08)),
            var(--bg-card);
          border: 1px solid rgba(var(--red-rgb), 0.3);
          border-radius: var(--radius-md);
          box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
          cursor: pointer;
          animation: fade-slide-up 0.2s var(--ease-out) both;
        }
        .toast-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--red);
          margin-top: 5px;
          flex-shrink: 0;
        }
        .toast-msg {
          font-size: 11.5px;
          line-height: 1.45;
          font-family: var(--font-mono);
          color: var(--red);
          word-break: break-word;
          user-select: text;
          -webkit-user-select: text;
        }
        /* Success variant — green tint (clean merges, etc.) */
        .toast--success {
          background:
            linear-gradient(rgba(var(--green-rgb), 0.08), rgba(var(--green-rgb), 0.08)),
            var(--bg-card);
          border-color: rgba(var(--green-rgb), 0.3);
        }
        .toast--success .toast-dot { background: var(--green); }
        .toast--success .toast-msg { color: var(--green); }
        @media (prefers-reduced-motion: reduce) {
          .toast { animation: none; }
        }
      `}</style>
    </>
  );
}
