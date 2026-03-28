import { Show } from "solid-js";
import type { Thread } from "../../types";
import { appStore } from "../../stores/app-store";

export function ThreadItem(props: { thread: Thread; isUncategorized: boolean }) {
  const { store, setStore, selectThread } = appStore;

  const isActive = () => store.activeTab === props.thread.id;
  const isRenaming = () => store.renamingThread?.id === props.thread.id;

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setStore("contextMenu", {
      type: "thread",
      id: props.thread.id,
      x: e.clientX,
      y: e.clientY,
    });
  }

  function handleDblClick() {
    setStore("renamingThread", { id: props.thread.id, text: props.thread.title });
  }

  function handleRenameSubmit(e: Event) {
    e.preventDefault();
    const text = store.renamingThread?.text?.trim();
    if (text) {
      import("../../ipc").then(({ renameThread }) => {
        renameThread(props.thread.id, text);
        setStore(
          "projects",
          (p) => p.threads.some((t) => t.id === props.thread.id),
          "threads",
          (t) => t.id === props.thread.id,
          "title",
          text
        );
      });
    }
    setStore("renamingThread", null);
  }

  function handleDragStart(e: DragEvent) {
    if (!props.isUncategorized) {
      e.preventDefault();
      return;
    }
    e.dataTransfer!.setData("text/thread-id", props.thread.id);
    e.dataTransfer!.effectAllowed = "move";
    setStore("draggingSidebarThread", props.thread.id);
  }

  function handleDragEnd() {
    setStore("draggingSidebarThread", null);
  }

  const dotColor = () => {
    if (props.thread.color) return props.thread.color;
    const status = store.sessionStatuses[props.thread.id];
    if (status === "ready") return "var(--green)";
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
            onInput={(e) =>
              setStore("renamingThread", "text", e.currentTarget.value)
            }
            onBlur={() => setStore("renamingThread", null)}
            autofocus
          />
        </form>
      }
    >
      <div
        class="thread-item"
        classList={{ active: isActive(), draggable: props.isUncategorized }}
        onClick={() => selectThread(props.thread.id)}
        onContextMenu={handleContextMenu}
        onDblClick={handleDblClick}
        draggable={props.isUncategorized}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
        <Show when={dotColor()}>
          <span class="status-dot" style={{ color: dotColor()! }}>
            &#x25CF;
          </span>
        </Show>
        <span
          class="thread-title"
          classList={{ "text-active": isActive() }}
          style={props.thread.color ? { "border-left": `2px solid ${props.thread.color}`, "padding-left": "6px" } : {}}
        >
          {props.thread.title}
        </span>
        <Show when={props.isUncategorized}>
          <span class="drag-handle">&#x2261;</span>
        </Show>
      </div>
    </Show>
  );
}

if (!document.getElementById("thread-item-styles")) {
  const style = document.createElement("style");
  style.id = "thread-item-styles";
  style.textContent = `
    .thread-item {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 5px 14px;
      margin: 0 4px;
      border-radius: var(--radius-sm);
      cursor: pointer;
      transition: background 0.12s;
      font-size: 13px;
      color: var(--text-secondary);
    }
    .thread-item:hover { background: var(--bg-muted); }
    .thread-item.active {
      background: var(--bg-accent);
      color: var(--text);
    }
    .thread-item.draggable { cursor: grab; }
    .thread-item.draggable:active { cursor: grabbing; }
    .status-dot { font-size: 7px; flex-shrink: 0; }
    .thread-title {
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .thread-title.text-active { color: var(--text); }
    .drag-handle {
      font-size: 12px;
      color: var(--text-tertiary);
      opacity: 0;
      transition: opacity 0.15s;
    }
    .thread-item:hover .drag-handle { opacity: 1; }
    .thread-rename {
      padding: 2px 14px;
      margin: 0 4px;
    }
    .thread-rename input {
      width: 100%;
      font-size: 13px;
    }
  `;
  document.head.appendChild(style);
}
