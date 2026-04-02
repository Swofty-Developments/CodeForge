import { For, Show, createSignal } from "solid-js";
import { DragDropProvider } from "@dnd-kit/solid";
import { useSortable } from "@dnd-kit/solid/sortable";
import { move } from "@dnd-kit/helpers";
import { appStore } from "../../stores/app-store";
import { browserClose } from "../../ipc";

function SortableTab(props: { tabId: string; index: number }) {
  const { store, setStore, closeTab, selectThread } = appStore;

  const thread = () =>
    store.projects.flatMap((p) => p.threads).find((t) => t.id === props.tabId);

  const isActive = () => store.activeTab === props.tabId;

  const color = () => {
    const t = thread();
    if (!t) return null;
    const project = store.projects.find((p) => p.threads.some((th) => th.id === t.id));
    return project?.color || null;
  };

  const statusDotColor = () => {
    if (props.tabId.startsWith("__")) return null;
    const status = store.sessionStatuses[props.tabId];
    if (status === "ready") return "var(--green)";
    if (status === "generating" || status === "starting") return "var(--sky)";
    if (status === "error") return "var(--red)";
    return null;
  };

  const isGenerating = () => {
    const status = store.sessionStatuses[props.tabId];
    return status === "generating" || status === "starting";
  };

  const isUnread = () => !isActive() && !!store.unreadTabs[props.tabId];

  const sortable = useSortable({
    get id() { return props.tabId; },
    get index() { return props.index; },
  });

  function handleClick() {
    if (props.tabId.startsWith("__")) {
      setStore("activeTab", props.tabId);
    } else {
      selectThread(props.tabId);
    }
  }

  return (
    <div
      ref={sortable.ref}
      class="tab"
      classList={{
        active: isActive(),
        dragging: typeof sortable.isDragging === 'function' ? sortable.isDragging() : !!sortable.isDragging,
        unread: isUnread(),
      }}
      style={color() ? { "border-bottom": `2px solid ${color()}` } : {}}
      onClick={handleClick}
    >
      <Show when={statusDotColor()}>
        {(dotColor) => (
          <span
            class="tab-status-dot"
            classList={{ pulsing: isGenerating() }}
            style={{ background: dotColor() }}
          />
        )}
      </Show>
      <span class="tab-label">{
        props.tabId === "__mcp__" ? "MCP Servers" :
        props.tabId === "__themes__" ? "Themes" :
        props.tabId === "__search__" ? "Search" :
        props.tabId === "__settings__" ? "Settings" :
        thread()?.title || "..."
      }</span>
      <Show when={isUnread()}>
        <span class="tab-unread-dot" />
      </Show>
      <button
        class="tab-close"
        onClick={(e) => { e.stopPropagation(); closeTab(props.tabId); }}
      >
        &times;
      </button>
    </div>
  );
}

export function TabBar() {
  const { store, setStore } = appStore;
  const [showCopied, setShowCopied] = createSignal(false);

  const browserOpen = () => {
    const tab = store.activeTab;
    return tab ? !!store.threadBrowserOpen[tab] : false;
  };

  function toggleBrowser() {
    const tab = store.activeTab;
    if (!tab) return;
    const opening = !store.threadBrowserOpen[tab];
    setStore("threadBrowserOpen", tab, opening);
    if (!opening) browserClose(tab).catch(() => {});
  }

  function toggleDiff() {
    const tab = store.activeTab;
    if (tab) setStore("threadDiffOpen", tab, !store.threadDiffOpen[tab]);
  }

  function exportChat() {
    const tab = store.activeTab;
    if (!tab) return;
    const msgs = store.threadMessages[tab];
    if (!msgs || msgs.length === 0) return;

    const thread = store.projects.flatMap((p) => p.threads).find((t) => t.id === tab);
    const title = thread?.title || "Chat";

    const md = `# ${title}\n\n` + msgs.map((m) => {
      const role = m.role === "user" ? "You" : m.role === "assistant" ? "Assistant" : "System";
      return `**${role}:**\n${m.content}\n`;
    }).join("\n---\n\n");

    navigator.clipboard.writeText(md).then(() => {
      setShowCopied(true);
      setTimeout(() => setShowCopied(false), 1500);
    });
  }

  return (
    <Show when={store.openTabs.length > 0}>
      <DragDropProvider
        onDragEnd={(event) => {
          setStore("openTabs", (tabs) => move(tabs, event));
        }}
      >
        <div class="tab-bar">
          <div class="tab-bar-tabs">
            <For each={store.openTabs}>
              {(tabId, idx) => <SortableTab tabId={tabId} index={idx()} />}
            </For>
          </div>
          <Show when={store.activeTab}>
            <div class="tab-bar-actions">
              <button
                class="tb-action"
                classList={{ active: browserOpen() }}
                onClick={toggleBrowser}
                title="Browser (Cmd+Shift+B)"
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z"/>
                </svg>
              </button>
              <button class="tb-action" onClick={exportChat} title="Export chat" style="position:relative;">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
                </svg>
                {showCopied() && <span class="tb-toast">Copied!</span>}
              </button>
              <button
                class="tb-action"
                classList={{ active: !!(store.activeTab && store.threadDiffOpen[store.activeTab]) }}
                onClick={toggleDiff}
                title="Diff view (Cmd+Shift+D)"
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="6" y1="3" x2="6" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 01-9 9"/>
                </svg>
              </button>
            </div>
          </Show>
        </div>
      </DragDropProvider>
    </Show>
  );
}

if (!document.getElementById("tab-bar-styles")) {
  const s = document.createElement("style");
  s.id = "tab-bar-styles";
  s.textContent = `
    .tab-bar {
      display: flex;
      align-items: stretch;
      padding: 6px 8px 0;
      background: var(--bg-muted);
      border-bottom: 1px solid var(--border);
      flex-shrink: 0;
    }
    .tab-bar-tabs {
      display: flex;
      align-items: stretch;
      gap: 1px;
      flex: 1;
      overflow-x: auto;
      overflow-y: hidden;
      min-width: 0;
    }
    .tab-bar-tabs::-webkit-scrollbar { height: 0; display: none; }
    .tab-bar-actions {
      display: flex;
      align-items: center;
      gap: 2px;
      padding: 0 2px 4px;
      flex-shrink: 0;
      margin-left: 4px;
    }
    .tb-url-group {
      display: flex;
      align-items: center;
      gap: 3px;
      padding: 0 4px 4px;
      margin-left: auto;
      min-width: 140px;
      max-width: 280px;
      flex-shrink: 1;
    }
    .tb-url {
      flex: 1;
      min-width: 0;
      height: 22px;
      padding: 0 6px;
      font-size: 11px;
      font-family: var(--font-mono);
      background: var(--bg-accent);
      border: 1px solid var(--border);
      border-radius: 4px;
      color: var(--text);
      outline: none;
    }
    .tb-url:focus { border-color: var(--primary); }
    .tb-url-go {
      height: 22px;
      padding: 0 8px;
      font-size: 10px;
      font-weight: 600;
      background: var(--primary);
      color: #fff;
      border-radius: 4px;
      flex-shrink: 0;
    }
    .tb-url-go:hover { filter: brightness(1.15); }
    .tb-action {
      width: 24px;
      height: 24px;
      border-radius: var(--radius-sm);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-tertiary);
      transition: background 0.1s, color 0.1s;
      position: relative;
    }
    .tb-action:hover {
      background: var(--bg-accent);
      color: var(--text-secondary);
    }
    .tb-action.active {
      color: var(--primary);
      background: var(--primary-glow);
    }
    .tb-toast {
      position: absolute;
      top: -24px;
      left: 50%;
      transform: translateX(-50%);
      background: var(--bg-accent);
      color: var(--green);
      font-size: 10px;
      font-weight: 600;
      padding: 2px 7px;
      border-radius: 4px;
      border: 1px solid var(--border);
      white-space: nowrap;
      pointer-events: none;
    }
    .tab {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 6px 12px;
      font-size: 12px;
      font-weight: 500;
      color: var(--text-secondary);
      border-radius: var(--radius-sm) var(--radius-sm) 0 0;
      cursor: grab;
      transition: background 0.12s, color 0.12s, opacity 0.15s;
      white-space: nowrap;
      user-select: none;
      flex-shrink: 0;
      touch-action: none;
    }
    .tab:active { cursor: grabbing; will-change: transform; }
    .tab:hover { background: var(--bg-hover); color: var(--text-secondary); }
    .tab.active {
      background: var(--bg-base);
      color: var(--text);
      border: 1px solid var(--border-strong);
      border-bottom: 1px solid var(--bg-base);
      margin-bottom: -1px;
    }
    .tab.dragging { opacity: 0.4; will-change: transform; }
    .tab-label {
      overflow: hidden;
      text-overflow: ellipsis;
      max-width: 120px;
    }
    .tab-close {
      color: var(--text-tertiary);
      padding: 2px;
      line-height: 1;
      border-radius: 3px;
      transition: background 0.12s, color 0.12s;
      cursor: pointer;
      display: flex;
      align-items: center;
    }
    .tab-close:hover {
      background: var(--bg-accent);
      color: var(--text);
    }
    .tab-status-dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      flex-shrink: 0;
      transition: background 0.3s;
    }
    .tab-status-dot.pulsing {
      animation: tab-pulse 1.5s ease-in-out infinite;
    }
    @keyframes tab-pulse {
      0%, 100% { opacity: 1; transform: scale(1); }
      50% { opacity: 0.5; transform: scale(0.8); }
    }
    .tab-unread-dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--primary);
      flex-shrink: 0;
    }
    .tab.unread .tab-label {
      color: var(--text);
      font-weight: 600;
    }
  `;
  document.head.appendChild(s);
}
