import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import { SplitPane } from "./SplitPane";

/**
 * SplitView renders two thread panes side by side with a draggable divider.
 *
 * Integration: Add this component in App.tsx to replace the main panel area
 * when `store.splitTab` is non-null:
 *
 *   <Show when={store.splitTab} fallback={<>{/* normal ChatArea + Composer *\/}</>}>
 *     <SplitView />
 *   </Show>
 *
 * Keyboard shortcut: Cmd+\ toggles split view.
 * Context menu: "Open in Split View" should call setSplitTab(threadId).
 */
export function SplitView() {
  const { store, setSplitTab } = appStore;

  const [splitPercent, setSplitPercent] = createSignal(50);
  const [dragging, setDragging] = createSignal(false);
  let containerRef: HTMLDivElement | undefined;

  function handleMouseDown(e: MouseEvent) {
    e.preventDefault();
    setDragging(true);
  }

  function handleMouseMove(e: MouseEvent) {
    if (!dragging() || !containerRef) return;
    const rect = containerRef.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const pct = Math.min(Math.max((x / rect.width) * 100, 15), 85);
    setSplitPercent(pct);
  }

  function handleMouseUp() {
    setDragging(false);
  }

  onMount(() => {
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  });

  onCleanup(() => {
    document.removeEventListener("mousemove", handleMouseMove);
    document.removeEventListener("mouseup", handleMouseUp);
  });

  // Keyboard shortcut: Cmd+\ to toggle split view
  function handleKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "\\") {
      e.preventDefault();
      setSplitTab(null);
    }
  }

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown);
  });
  onCleanup(() => {
    document.removeEventListener("keydown", handleKeyDown);
  });

  return (
    <div
      class="split-view-container"
      ref={containerRef}
      classList={{ "split-dragging": dragging() }}
    >
      <div
        class="split-view-pane"
        style={{ width: `${splitPercent()}%` }}
      >
        <Show when={store.activeTab}>
          <SplitPane threadId={store.activeTab!} />
        </Show>
      </div>

      <div
        class="split-view-divider"
        onMouseDown={handleMouseDown}
      >
        <div class="split-view-divider-line" />
      </div>

      <div
        class="split-view-pane"
        style={{ width: `${100 - splitPercent()}%` }}
      >
        <Show when={store.splitTab}>
          <div class="split-view-pane-wrapper">
            <SplitPane threadId={store.splitTab!} />
            <button
              class="split-view-close"
              onClick={() => setSplitTab(null)}
              title="Close Split View (Cmd+\\)"
            >
              Close Split
            </button>
          </div>
        </Show>
      </div>
    </div>
  );
}

if (!document.getElementById("split-view-styles")) {
  const style = document.createElement("style");
  style.id = "split-view-styles";
  style.textContent = `
    .split-view-container {
      display: flex;
      flex: 1;
      min-height: 0;
      overflow: hidden;
      background: #131316;
    }
    .split-view-container.split-dragging {
      cursor: col-resize;
      user-select: none;
    }

    .split-view-pane {
      display: flex;
      flex-direction: column;
      min-width: 0;
      overflow: hidden;
      position: relative;
    }

    .split-view-pane-wrapper {
      display: flex;
      flex-direction: column;
      height: 100%;
      position: relative;
    }

    .split-view-divider {
      width: 4px;
      cursor: col-resize;
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
      z-index: 2;
    }
    .split-view-divider:hover .split-view-divider-line,
    .split-dragging .split-view-divider-line {
      background: #6680f2;
    }
    .split-view-divider-line {
      width: 1px;
      height: 100%;
      background: rgba(255,255,255,0.07);
      transition: background 0.15s;
    }

    .split-view-close {
      position: absolute;
      top: 6px;
      right: 8px;
      z-index: 5;
      font-size: 11px;
      color: rgba(255,255,255,0.5);
      background: rgba(255,255,255,0.06);
      border: 1px solid rgba(255,255,255,0.1);
      border-radius: 6px;
      padding: 3px 10px;
      cursor: pointer;
      transition: all 0.15s;
    }
    .split-view-close:hover {
      background: rgba(255,255,255,0.1);
      color: rgba(255,255,255,0.8);
      border-color: rgba(255,255,255,0.2);
    }
  `;
  document.head.appendChild(style);
}
