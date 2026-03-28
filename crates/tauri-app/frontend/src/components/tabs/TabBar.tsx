import { For, Show } from "solid-js";
import { appStore } from "../../stores/app-store";

export function TabBar() {
  const { store, setStore, closeTab, reorderTabs } = appStore;

  function findThread(id: string) {
    return store.projects.flatMap((p) => p.threads).find((t) => t.id === id);
  }

  function handleDragStart(e: DragEvent, tabId: string) {
    e.dataTransfer!.setData("text/tab-id", tabId);
    e.dataTransfer!.effectAllowed = "move";
    setStore("draggingTab", tabId);
  }

  function handleDragOver(e: DragEvent, idx: number) {
    e.preventDefault();
    e.dataTransfer!.dropEffect = "move";
    const tabId = store.draggingTab;
    if (tabId) reorderTabs(tabId, idx);
  }

  function handleDragEnd() {
    setStore("draggingTab", null);
  }

  return (
    <Show when={store.openTabs.length > 0}>
      <div class="tab-bar">
        <For each={store.openTabs}>
          {(tabId, idx) => {
            const thread = () => findThread(tabId);
            const isActive = () => store.activeTab === tabId;
            const isDragging = () => store.draggingTab === tabId;
            const color = () => thread()?.color;

            return (
              <div
                class="tab"
                classList={{
                  active: isActive(),
                  dragging: isDragging(),
                }}
                style={color() ? { "border-bottom": `2px solid ${color()}` } : {}}
                draggable={true}
                onDragStart={(e) => handleDragStart(e, tabId)}
                onDragOver={(e) => handleDragOver(e, idx())}
                onDragEnd={handleDragEnd}
                onClick={() => setStore("activeTab", tabId)}
              >
                <span class="tab-label">{thread()?.title || "..."}</span>
                <button
                  class="tab-close"
                  onClick={(e) => {
                    e.stopPropagation();
                    closeTab(tabId);
                  }}
                >
                  &times;
                </button>
              </div>
            );
          }}
        </For>
      </div>
    </Show>
  );
}

if (!document.getElementById("tab-bar-styles")) {
  const style = document.createElement("style");
  style.id = "tab-bar-styles";
  style.textContent = `
    .tab-bar {
      display: flex;
      gap: 1px;
      padding: 6px 8px 0;
      background: var(--bg-muted);
      border-bottom: 1px solid var(--border);
      overflow-x: auto;
      flex-shrink: 0;
    }
    .tab {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 6px 12px;
      font-size: 12px;
      color: var(--text-secondary);
      border-radius: var(--radius-sm) var(--radius-sm) 0 0;
      cursor: pointer;
      transition: background 0.12s, color 0.12s;
      white-space: nowrap;
      user-select: none;
    }
    .tab:hover { background: var(--bg-accent); }
    .tab.active {
      background: var(--bg-base);
      color: var(--text);
      border: 1px solid var(--border-strong);
      border-bottom: 1px solid var(--bg-base);
      margin-bottom: -1px;
    }
    .tab.dragging { opacity: 0.5; }
    .tab-label {
      overflow: hidden;
      text-overflow: ellipsis;
      max-width: 120px;
    }
    .tab-close {
      font-size: 14px;
      color: var(--text-tertiary);
      padding: 0 2px;
      line-height: 1;
      border-radius: 3px;
      transition: background 0.12s, color 0.12s;
    }
    .tab-close:hover {
      background: var(--bg-accent);
      color: var(--text);
    }
  `;
  document.head.appendChild(style);
}
