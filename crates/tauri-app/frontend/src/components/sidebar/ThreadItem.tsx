import { Show } from "solid-js";
import type { Thread } from "../../types";
import { appStore } from "../../stores/app-store";

export function ThreadItem(props: {
  thread: Thread;
  isUncategorized: boolean;
  groupColor: string | null;
  sortableRef?: (el: HTMLElement) => void;
  isDragging?: boolean;
  prNumber?: number;
  hasWorktree?: boolean;
}) {
  const { store, setStore, selectThread } = appStore;

  const isActive = () => store.activeTab === props.thread.id;
  const isRenaming = () => store.renamingThread?.id === props.thread.id;

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setStore("contextMenu", { type: "thread", id: props.thread.id, x: e.clientX, y: e.clientY });
  }

  function handleRenameSubmit(e: Event) {
    e.preventDefault();
    const text = store.renamingThread?.text?.trim();
    if (text) {
      import("../../ipc").then(({ renameThread }) => {
        renameThread(props.thread.id, text);
        setStore("projects", (projects) =>
          projects.map((p) => ({
            ...p,
            threads: p.threads.map((t) => t.id === props.thread.id ? { ...t, title: text } : t),
          }))
        );
      });
    }
    setStore("renamingThread", null);
  }

  const statusColor = () => {
    const status = store.sessionStatuses[props.thread.id];
    if (!status || status === "idle") return null;
    if (status === "ready") return props.groupColor || "var(--green)";
    if (status === "generating" || status === "starting") return "var(--sky)";
    if (status === "error") return "var(--red)";
    return null;
  };

  return (
    <Show
      when={!isRenaming()}
      fallback={
        <form class="thread-rename" onSubmit={handleRenameSubmit}>
          <input
            value={store.renamingThread?.text || ""}
            onInput={(e) => setStore("renamingThread", "text", e.currentTarget.value)}
            onBlur={handleRenameSubmit}
            autofocus
          />
        </form>
      }
    >
      <div
        ref={props.sortableRef}
        class="thread-item"
        classList={{
          active: isActive(),
          dragging: !!props.isDragging,
          "can-drag": props.isUncategorized,
        }}
        onClick={() => selectThread(props.thread.id)}
        onContextMenu={handleContextMenu}
        onDblClick={() => setStore("renamingThread", { id: props.thread.id, text: props.thread.title })}
      >
        <span
          class="thread-title"
          style={props.groupColor ? { "border-left": `2px solid ${props.groupColor}`, "padding-left": "6px" } : {}}
        >
          {props.thread.title}
        </span>
        <Show when={props.prNumber}>
          <span class="thread-pr-badge">PR #{props.prNumber}</span>
        </Show>
        <Show when={props.hasWorktree}>
          <svg class="thread-wt-icon" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--primary)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M6 21V9a9 9 0 009 9" />
          </svg>
        </Show>
        <div class="thread-right">
          <Show when={statusColor()}>
            <span class="status-dot" style={{ background: statusColor()! }} />
          </Show>
          <div class="thread-actions">
            <button
              class="thread-action-btn"
              onClick={(e) => { e.stopPropagation(); setStore("renamingThread", { id: props.thread.id, text: props.thread.title }); }}
              title="Rename"
            >
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M17 3a2.83 2.83 0 114 4L7.5 20.5 2 22l1.5-5.5L17 3z" />
              </svg>
            </button>
            <button
              class="thread-action-btn delete"
              onClick={(e) => {
                e.stopPropagation();
                import("../../ipc").then(({ deleteThread }) => {
                  deleteThread(props.thread.id);
                  setStore("projects", (projects) =>
                    projects.map((p) => ({ ...p, threads: p.threads.filter((t) => t.id !== props.thread.id) }))
                  );
                  setStore("openTabs", (tabs) => tabs.filter((t) => t !== props.thread.id));
                  if (store.activeTab === props.thread.id) {
                    setStore("activeTab", store.openTabs.filter((t) => t !== props.thread.id).pop() || null);
                  }
                });
              }}
              title="Delete"
            >
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
            <Show when={props.isUncategorized}>
              <svg class="drag-hint" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                <line x1="8" y1="6" x2="16" y2="6" /><line x1="8" y1="12" x2="16" y2="12" /><line x1="8" y1="18" x2="16" y2="18" />
              </svg>
            </Show>
          </div>
        </div>
      </div>
    </Show>
  );
}

if (!document.getElementById("thread-item-styles")) {
  const s = document.createElement("style");
  s.id = "thread-item-styles";
  s.textContent = `
    .thread-item {
      display: flex; align-items: center; gap: 8px;
      padding: 5px 12px; margin: 1px 0;
      border-radius: var(--radius-sm); cursor: pointer;
      transition: background 0.15s, color 0.15s;
      font-size: 13px; color: var(--text-secondary);
      touch-action: none;
      position: relative;
    }
    .thread-item[aria-disabled="true"] {
      opacity: 1;
      cursor: pointer;
    }
    .thread-item:hover { background: var(--bg-hover); color: var(--text); }
    .thread-item.active {
      background: var(--bg-accent);
      color: var(--text);
    }
    .thread-item.active::before {
      content: "";
      position: absolute;
      left: 0;
      top: 4px;
      bottom: 4px;
      width: 2px;
      border-radius: 1px;
      background: var(--primary);
    }
    .thread-item.dragging { opacity: 0.3; }
    .thread-item.can-drag { cursor: grab; }
    .thread-item.can-drag:active { cursor: grabbing; }
    .thread-title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-secondary); }
    .thread-item.active .thread-title { color: var(--text); }
    .thread-right {
      display: flex;
      align-items: center;
      gap: 4px;
      flex-shrink: 0;
    }
    .status-dot {
      width: 6px; height: 6px;
      border-radius: 50%;
      flex-shrink: 0;
      transition: background 0.3s;
    }
    .thread-actions {
      display: flex; align-items: center; gap: 2px;
      opacity: 0; transition: opacity 0.12s;
    }
    .thread-item:hover .thread-actions,
    .thread-item.active .thread-actions { opacity: 1; }
    .thread-action-btn {
      color: var(--text-tertiary);
      padding: 3px;
      border-radius: 4px;
      transition: background 0.1s, color 0.1s;
      display: flex;
      align-items: center;
    }
    .thread-action-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }
    .thread-action-btn.delete:hover { color: var(--red); }
    .drag-hint { color: var(--text-tertiary); }
    .thread-pr-badge {
      font-size: 9px;
      font-weight: 600;
      font-family: var(--font-mono);
      color: var(--primary);
      background: rgba(107, 124, 255, 0.1);
      padding: 1px 5px;
      border-radius: var(--radius-pill);
      white-space: nowrap;
      flex-shrink: 0;
    }
    .thread-wt-icon {
      flex-shrink: 0;
      opacity: 0.7;
    }
    .thread-rename { padding: 2px 12px; margin: 1px 0; }
    .thread-rename input { width: 100%; font-size: 13px; }
  `;
  document.head.appendChild(s);
}
