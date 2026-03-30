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

  function handleSetGroupColor(color: string | null) {
    const m = menu();
    if (!m || m.type !== "project") return;
    setStore("projects", (projects) =>
      projects.map((p) => (p.id === m.id ? { ...p, color } : p))
    );
    if (color) {
      ipc.setSetting(`project_color:${m.id}`, color);
    } else {
      ipc.setSetting(`project_color:${m.id}`, "");
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
      setStore("projects", (projects) =>
        projects.map((p) => ({
          ...p,
          threads: p.threads.filter((t) => t.id !== m.id),
        }))
      );
      setStore("openTabs", (tabs) => tabs.filter((t) => t !== m.id));
      if (store.activeTab === m.id) {
        setStore("activeTab", store.openTabs[store.openTabs.length - 1] || null);
      }
    } else {
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

  const isProject = () => menu()?.type === "project";

  return (
    <Show when={menu()}>
      <div class="context-backdrop" onClick={close}>
        <div
          class="context-menu"
          style={{ left: `${menu()!.x}px`, top: `${menu()!.y}px` }}
          onClick={(e) => e.stopPropagation()}
        >
          <button class="ctx-item" onClick={handleRename}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M17 3a2.83 2.83 0 114 4L7.5 20.5 2 22l1.5-5.5L17 3z" />
            </svg>
            Rename
          </button>
          <button class="ctx-item danger" onClick={handleDelete}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2" />
            </svg>
            Delete
          </button>

          <Show when={isProject()}>
            <div class="ctx-divider" />
            <div class="ctx-color-label">Group Color</div>
            <div class="ctx-colors">
              {THREAD_COLORS.map((c) => (
                <button
                  class="color-dot"
                  style={{ background: c.hex }}
                  onClick={() => handleSetGroupColor(c.hex)}
                  title={c.label}
                />
              ))}
              <button
                class="color-dot clear"
                onClick={() => handleSetGroupColor(null)}
                title="Clear color"
              >
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                  <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
          </Show>
        </div>
      </div>

      <style>{`
        .context-backdrop {
          position: fixed;
          inset: 0;
          z-index: 200;
        }
        @keyframes ctx-menu-in {
          from { opacity: 0; transform: scale(0.94) translateY(-2px); }
          to { opacity: 1; transform: scale(1) translateY(0); }
        }
        .context-menu {
          position: fixed;
          background: var(--bg-card);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-md);
          padding: 4px;
          min-width: 170px;
          z-index: 201;
          box-shadow: 0 12px 32px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.03);
          backdrop-filter: blur(12px);
          -webkit-backdrop-filter: blur(12px);
          animation: ctx-menu-in 0.12s cubic-bezier(0.16, 1, 0.3, 1);
          transform-origin: top left;
        }
        .ctx-item {
          display: flex;
          align-items: center;
          gap: 8px;
          width: 100%;
          padding: 7px 12px;
          font-size: 12px;
          font-weight: 500;
          text-align: left;
          border-radius: var(--radius-sm);
          transition: background 0.1s;
          color: var(--text-secondary);
        }
        .ctx-item svg { color: var(--text-tertiary); flex-shrink: 0; }
        .ctx-item:hover { background: var(--bg-accent); color: var(--text); }
        .ctx-item:hover svg { color: var(--text-secondary); }
        .ctx-item.danger { color: var(--red); }
        .ctx-item.danger svg { color: var(--red); opacity: 0.7; }
        .ctx-item.danger:hover { background: rgba(242, 95, 103, 0.08); }
        .ctx-divider {
          height: 1px;
          background: var(--border);
          margin: 4px 0;
        }
        .ctx-color-label {
          font-size: 10px;
          font-weight: 600;
          color: var(--text-tertiary);
          padding: 4px 10px 2px;
          text-transform: uppercase;
          letter-spacing: 0.06em;
        }
        .ctx-colors {
          display: flex;
          gap: 3px;
          padding: 4px 8px 6px;
          flex-wrap: wrap;
        }
        .color-dot {
          width: 18px;
          height: 18px;
          border-radius: 50%;
          border: 2px solid transparent;
          transition: transform 0.1s, border-color 0.1s;
          cursor: pointer;
        }
        .color-dot:hover { transform: scale(1.15); border-color: rgba(255,255,255,0.2); }
        .color-dot.clear {
          background: var(--bg-accent);
          border: 1px solid var(--border);
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .color-dot.clear svg { color: var(--text-tertiary); }
      `}</style>
    </Show>
  );
}
