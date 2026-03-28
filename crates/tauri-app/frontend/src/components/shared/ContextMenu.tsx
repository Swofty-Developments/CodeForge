import { Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import { THREAD_COLORS } from "../../types";
import * as ipc from "../../ipc";

export function ContextMenu() {
  const { store, setStore } = appStore;
  const menu = () => store.contextMenu;

  function close() {
    setStore("contextMenu", null);
  }

  function handleSetColor(color: string | null) {
    const m = menu();
    if (!m) return;
    if (m.type === "thread") {
      ipc.setThreadColor(m.id, color);
      setStore(
        "projects",
        (p) => p.threads.some((t) => t.id === m.id),
        "threads",
        (t) => t.id === m.id,
        "color",
        color
      );
    }
    close();
  }

  function handleRename() {
    const m = menu();
    if (!m) return;
    if (m.type === "thread") {
      const thread = store.projects.flatMap((p) => p.threads).find((t) => t.id === m.id);
      setStore("renamingThread", { id: m.id, text: thread?.title || "" });
    } else {
      const project = store.projects.find((p) => p.id === m.id);
      setStore("renamingProject", { id: m.id, text: project?.name || "" });
    }
    close();
  }

  function handleDelete() {
    const m = menu();
    if (!m) return;
    if (m.type === "thread") {
      ipc.deleteThread(m.id);
      setStore("projects", (p) => ({
        ...p,
        threads: p.threads.filter((t) => t.id !== m.id),
      }));
      setStore("openTabs", (tabs) => tabs.filter((t) => t !== m.id));
      if (store.activeTab === m.id) {
        setStore("activeTab", store.openTabs[store.openTabs.length - 1] || null);
      }
    } else {
      // For project deletion, just delete the project and keep threads uncategorized
      ipc.deleteProject(m.id, false);
      const threads = store.projects.find((p) => p.id === m.id)?.threads || [];
      setStore("projects", (projects) => {
        const filtered = projects.filter((p) => p.id !== m.id);
        const uncatIdx = filtered.findIndex((p) => p.path === ".");
        if (uncatIdx !== -1 && threads.length > 0) {
          filtered[uncatIdx] = {
            ...filtered[uncatIdx],
            threads: [...filtered[uncatIdx].threads, ...threads],
          };
        }
        return filtered;
      });
    }
    close();
  }

  return (
    <Show when={menu()}>
      <div class="context-backdrop" onClick={close}>
        <div
          class="context-menu"
          style={{ left: `${menu()!.x}px`, top: `${menu()!.y}px` }}
          onClick={(e) => e.stopPropagation()}
        >
          <button class="ctx-item" onClick={handleRename}>
            Rename
          </button>
          <button class="ctx-item danger" onClick={handleDelete}>
            Delete
          </button>
          <div class="ctx-divider" />
          <div class="ctx-colors">
            {THREAD_COLORS.map((c) => (
              <button
                class="color-dot"
                style={{ color: c.hex }}
                onClick={() => handleSetColor(c.hex)}
                title={c.label}
              >
                &#x25CF;
              </button>
            ))}
            <button
              class="color-dot clear"
              onClick={() => handleSetColor(null)}
              title="Clear color"
            >
              &times;
            </button>
          </div>
        </div>
      </div>

      <style>{`
        .context-backdrop {
          position: fixed;
          inset: 0;
          z-index: 200;
        }
        .context-menu {
          position: fixed;
          background: var(--bg-card);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-md);
          padding: 4px;
          min-width: 160px;
          z-index: 201;
          box-shadow: 0 8px 24px rgba(0,0,0,0.4);
        }
        .ctx-item {
          display: block;
          width: 100%;
          padding: 6px 12px;
          font-size: 12px;
          text-align: left;
          border-radius: var(--radius-sm);
          transition: background 0.1s;
          color: var(--text);
        }
        .ctx-item:hover { background: var(--bg-accent); }
        .ctx-item.danger { color: var(--red); }
        .ctx-divider {
          height: 1px;
          background: var(--border);
          margin: 4px 0;
        }
        .ctx-colors {
          display: flex;
          gap: 2px;
          padding: 4px 8px;
          flex-wrap: wrap;
        }
        .color-dot {
          font-size: 16px;
          padding: 2px;
          border-radius: var(--radius-sm);
          transition: background 0.1s;
          line-height: 1;
        }
        .color-dot:hover { background: var(--bg-accent); }
        .color-dot.clear { color: var(--text-tertiary); font-size: 14px; }
      `}</style>
    </Show>
  );
}
