/* App shell: sidebar (feature tree) · main panel (tab bar / view / status bar) ·
 * right session pane · Cmd+K palette. Layout per docs/ARCHITECTURE.md §Frontend. */

import { Match, Show, Switch, onCleanup, onMount } from "solid-js";
import { CommandPalette } from "./components/CommandPalette";
import { SessionPane } from "./components/SessionPane";
import { Sidebar } from "./components/Sidebar";
import { StatusBar } from "./components/StatusBar";
import { TabBar } from "./components/TabBar";
import { DiffReview } from "./views/DiffReview";
import { FeatureDetail } from "./views/FeatureDetail";
import { TimelineView } from "./views/TimelineView";
import { Welcome } from "./views/Welcome";
import { appStore } from "./stores/app-store";

export default function App() {
  const { store } = appStore;

  let sidebarDragging = false;

  function onSidebarDragStart(e: MouseEvent) {
    e.preventDefault();
    sidebarDragging = true;
    document.body.style.cursor = "col-resize";
  }

  function onMouseMove(e: MouseEvent) {
    if (sidebarDragging) appStore.setSidebarWidth(e.clientX);
  }

  function onMouseUp() {
    if (sidebarDragging) {
      sidebarDragging = false;
      document.body.style.cursor = "";
    }
  }

  function onKeyDown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      appStore.setPaletteOpen(!store.paletteOpen);
    } else if (e.key === "Escape" && store.paletteOpen) {
      appStore.setPaletteOpen(false);
    } else if (mod && e.key === "\\") {
      e.preventDefault();
      appStore.toggleSessionPane();
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
      <Sidebar />
      <div class="resize-handle" onMouseDown={onSidebarDragStart} />
      <div class="main-panel">
        <Show when={store.activeView !== "welcome"} fallback={<Welcome />}>
          <TabBar />
          <div class="main-panel-body">
            <div class="main-panel-view">
              <Switch>
                <Match when={store.activeView === "feature"}>
                  <FeatureDetail />
                </Match>
                <Match when={store.activeView === "timeline"}>
                  <TimelineView />
                </Match>
                <Match when={store.activeView === "diff"}>
                  <DiffReview />
                </Match>
              </Switch>
            </div>
            <Show when={store.sessionPaneOpen}>
              <SessionPane />
            </Show>
          </div>
          <StatusBar />
        </Show>
      </div>
      <Show when={store.paletteOpen}>
        <CommandPalette />
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
        .resize-handle {
          width: 5px;
          cursor: col-resize;
          background: transparent;
          flex-shrink: 0;
          position: relative;
        }
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
      `}</style>
    </>
  );
}
