import { For, Show, createSignal, createMemo, createEffect, on, onCleanup } from "solid-js";
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

  // Project color for grouping tint
  const projectColor = () => {
    const t = thread();
    if (!t) return null;
    const project = store.projects.find((p) => p.threads.some((th) => th.id === t.id));
    return project?.color || null;
  };

  const statusDotColor = () => {
    if (props.tabId.startsWith("__")) return null;
    const status = store.runStates[props.tabId];
    if (status === "ready") return "var(--green)";
    if (status === "generating" || status === "starting") return "var(--sky)";
    if (status === "error") return "var(--red)";
    return null;
  };

  const isGenerating = () => {
    const status = store.runStates[props.tabId];
    return status === "generating" || status === "starting";
  };

  const isUnread = () => !isActive() && !!store.unreadTabs[props.tabId];

  // Detect when a background thread just completed (for breathe effect)
  const [justCompleted, setJustCompleted] = createSignal(false);
  let prevStatus: string | undefined;
  createEffect(on(
    () => store.runStates[props.tabId],
    (status) => {
      if (prevStatus === "generating" && status === "ready" && !isActive()) {
        setJustCompleted(true);
        setTimeout(() => setJustCompleted(false), 1500);
      }
      prevStatus = status;
    }
  ));

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

  // Tab hover preview — show last assistant message
  const [hovered, setHovered] = createSignal(false);
  const [previewPos, setPreviewPos] = createSignal({ x: 0, y: 0 });
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  let tabEl: HTMLDivElement | undefined;

  function onEnter() {
    if (isActive()) return;
    hoverTimer = setTimeout(() => {
      if (tabEl) {
        const rect = tabEl.getBoundingClientRect();
        setPreviewPos({ x: rect.left + rect.width / 2, y: rect.bottom + 6 });
      }
      setHovered(true);
    }, 350);
  }
  function onLeave() {
    clearTimeout(hoverTimer);
    setHovered(false);
  }

  const lastAssistantMsg = createMemo(() => {
    if (props.tabId.startsWith("__")) return null;
    const msgs = store.threadMessages[props.tabId];
    if (!msgs) return null;
    for (let i = msgs.length - 1; i >= 0; i--) {
      if (msgs[i].role === "assistant" && msgs[i].content) {
        return msgs[i].content.slice(0, 120) + (msgs[i].content.length > 120 ? "..." : "");
      }
    }
    return null;
  });

  // Compute project color tint style
  const tintStyle = () => {
    const c = projectColor();
    if (!c || isActive()) return {};
    return { "border-bottom": `2px solid ${c}` };
  };

  return (
    <div
      ref={(el) => { sortable.ref(el); tabEl = el; }}
      class="tab"
      classList={{
        active: isActive(),
        dragging: typeof sortable.isDragging === 'function' ? sortable.isDragging() : !!sortable.isDragging,
        unread: isUnread(),
        "tab-breathe": justCompleted(),
      }}
      style={tintStyle()}
      onClick={handleClick}
      onMouseEnter={onEnter}
      onMouseLeave={onLeave}
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
        props.tabId === "__skills__" ? "Skills" :
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

      {/* Hover preview tooltip — fixed position to escape overflow clipping */}
      <Show when={hovered() && lastAssistantMsg()}>
        <div
          class="tab-preview"
          style={{
            left: `${previewPos().x}px`,
            top: `${previewPos().y}px`,
          }}
        >
          <div class="tab-preview-text">{lastAssistantMsg()}</div>
        </div>
      </Show>
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

  // Detect overflow for fade edges
  let tabsRef: HTMLDivElement | undefined;
  const [overflowLeft, setOverflowLeft] = createSignal(false);
  const [overflowRight, setOverflowRight] = createSignal(false);

  function checkOverflow() {
    if (!tabsRef) return;
    setOverflowLeft(tabsRef.scrollLeft > 4);
    setOverflowRight(tabsRef.scrollLeft < tabsRef.scrollWidth - tabsRef.clientWidth - 4);
  }

  // Check on mount and when tabs change
  createEffect(() => {
    const _ = store.openTabs.length;
    requestAnimationFrame(checkOverflow);
  });

  return (
    <Show when={store.openTabs.length > 0}>
      <>
      <DragDropProvider
        onDragEnd={(event) => {
          setStore("openTabs", (tabs) => move(tabs, event));
        }}
      >
        <div class="tab-bar">
          <div
            class="tab-bar-tabs"
            classList={{
              "overflow-left": overflowLeft(),
              "overflow-right": overflowRight(),
            }}
          >
            <div
              class="tab-bar-tabs-inner"
              ref={tabsRef}
              onScroll={checkOverflow}
            >
              <For each={store.openTabs}>
                {(tabId, idx) => <SortableTab tabId={tabId} index={idx()} />}
              </For>
            </div>
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
      <style>{`
    .tab-bar {
      display: flex;
      align-items: stretch;
      padding: 6px 8px 0;
      background: var(--bg-muted);
      border-bottom: 1px solid var(--border);
      flex-shrink: 0;
    }

    /* ── Scrollable tabs with fade edges ── */
    .tab-bar-tabs {
      flex: 1;
      min-width: 0;
      position: relative;
    }
    .tab-bar-tabs::before,
    .tab-bar-tabs::after {
      content: "";
      position: absolute;
      top: 0;
      bottom: 0;
      width: 24px;
      z-index: 3;
      pointer-events: none;
      opacity: 0;
      transition: opacity 0.15s;
    }
    .tab-bar-tabs::before {
      left: 0;
      background: linear-gradient(to right, var(--bg-muted), transparent);
    }
    .tab-bar-tabs::after {
      right: 0;
      background: linear-gradient(to left, var(--bg-muted), transparent);
    }
    .tab-bar-tabs.overflow-left::before { opacity: 1; }
    .tab-bar-tabs.overflow-right::after { opacity: 1; }

    .tab-bar-tabs-inner {
      display: flex;
      align-items: stretch;
      gap: 1px;
      overflow-x: auto;
      overflow-y: hidden;
      scroll-behavior: smooth;
    }
    .tab-bar-tabs-inner::-webkit-scrollbar { height: 0; display: none; }

    .tab-bar-actions {
      display: flex;
      align-items: center;
      gap: 2px;
      padding: 0 2px 4px;
      flex-shrink: 0;
      margin-left: 4px;
    }
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

    /* ── Tab ── */
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
      transition: background 0.15s ease, color 0.15s ease, opacity 0.15s ease,
                  box-shadow 0.2s ease, transform 0.12s ease;
      white-space: nowrap;
      user-select: none;
      flex-shrink: 0;
      touch-action: none;
      position: relative;
    }
    .tab:active { cursor: grabbing; will-change: transform; transform: scale(1.02); }
    .tab:hover {
      background: var(--bg-hover);
      color: var(--text-secondary);
    }
    .tab.active {
      background: var(--bg-base);
      color: var(--text);
      border: 1px solid var(--border-strong);
      border-bottom: 1px solid var(--bg-base);
      margin-bottom: -1px;
      box-shadow: 0 -1px 8px rgba(0, 0, 0, 0.1);
    }
    .tab.dragging {
      opacity: 0.85;
      will-change: transform;
      transform: scale(1.03);
      box-shadow: 0 4px 12px rgba(0,0,0,0.3);
      z-index: 10;
    }

    /* ── Background thread completion breathe ── */
    .tab-breathe {
      animation: tab-breathe-anim 1.5s ease-out;
    }
    @keyframes tab-breathe-anim {
      0% { box-shadow: 0 0 0 0 rgba(76, 214, 148, 0.4); }
      30% { box-shadow: 0 0 12px 2px rgba(76, 214, 148, 0.3); transform: scale(1.03); }
      100% { box-shadow: 0 0 0 0 rgba(76, 214, 148, 0); transform: scale(1); }
    }

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

    /* ── Hover preview tooltip (fixed to escape overflow clipping) ── */
    .tab-preview {
      position: fixed;
      transform: translateX(-50%);
      z-index: 999;
      width: 220px;
      max-width: 280px;
      background: var(--bg-card);
      border: 1px solid var(--border-strong);
      border-radius: var(--radius-md);
      padding: 8px 10px;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
      animation: tab-preview-in 0.12s ease-out both;
      pointer-events: none;
    }
    .tab-preview::before {
      content: "";
      position: absolute;
      top: -4px;
      left: 50%;
      transform: translateX(-50%) rotate(45deg);
      width: 8px;
      height: 8px;
      background: var(--bg-card);
      border-left: 1px solid var(--border-strong);
      border-top: 1px solid var(--border-strong);
    }
    @keyframes tab-preview-in {
      from { opacity: 0; transform: translateY(4px); }
      to { opacity: 1; transform: translateY(0); }
    }
    .tab-preview-text {
      font-size: 11px;
      line-height: 1.45;
      color: var(--text-secondary);
      overflow: hidden;
      display: -webkit-box;
      -webkit-line-clamp: 4;
      -webkit-box-orient: vertical;
      font-family: var(--font-body);
      font-weight: 400;
      white-space: normal;
    }

    /* ── prefers-reduced-motion ── */
    @media (prefers-reduced-motion: reduce) {
      .tab-breathe { animation: none; }
      .tab-preview { animation: none; }
      .tab, .tab-status-dot { transition: none; }
    }
      `}</style>
      </>
    </Show>
  );
}
