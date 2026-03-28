import { For, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import { ProjectGroup } from "./ProjectGroup";

export function Sidebar() {
  const { store, setStore, newThread } = appStore;

  let sidebarRef: HTMLDivElement | undefined;
  let dragging = false;

  function startResize(e: MouseEvent) {
    dragging = true;
    e.preventDefault();
    const onMove = (ev: MouseEvent) => {
      if (!dragging) return;
      const w = Math.max(180, Math.min(500, ev.clientX));
      setStore("sidebarWidth", w);
    };
    const onUp = () => {
      dragging = false;
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
    };
    document.body.style.cursor = "col-resize";
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  // Sort: categorized first, uncategorized last
  const sortedProjects = () => {
    return [...store.projects].sort((a, b) => {
      if (a.path === "." && b.path !== ".") return 1;
      if (a.path !== "." && b.path === ".") return -1;
      return 0;
    });
  };

  return (
    <>
      <div
        class="sidebar"
        ref={sidebarRef}
        style={{ width: `${store.sidebarWidth}px` }}
      >
        <div class="sidebar-header">
          <span class="sidebar-title">CodeForge</span>
          <button
            class="icon-btn"
            onClick={() => setStore("settingsOpen", true)}
            title="Settings"
          >
            &#x2699;
          </button>
        </div>

        <div class="sidebar-content">
          <Show
            when={store.projects.length > 0}
            fallback={
              <div class="empty-state">
                <p>No threads yet</p>
                <p class="hint">Click + below to start</p>
              </div>
            }
          >
            <For each={sortedProjects()}>
              {(project) => <ProjectGroup project={project} />}
            </For>
          </Show>
        </div>

        <div class="sidebar-footer">
          <button class="new-thread-btn" onClick={() => newThread()}>
            <span class="plus">+</span> New Thread
          </button>
        </div>
      </div>

      <div class="resize-handle" onMouseDown={startResize} />

      <style>{`
        .sidebar {
          display: flex;
          flex-direction: column;
          background: var(--bg-surface);
          border-right: 1px solid var(--border);
          flex-shrink: 0;
          height: 100vh;
        }
        .sidebar-header {
          display: flex;
          align-items: center;
          padding: 14px 16px;
          justify-content: space-between;
        }
        .sidebar-title {
          font-size: 15px;
          font-weight: 600;
          color: var(--text);
        }
        .icon-btn {
          font-size: 16px;
          color: var(--text-tertiary);
          padding: 4px 6px;
          border-radius: var(--radius-sm);
          transition: background 0.15s;
        }
        .icon-btn:hover {
          background: var(--bg-accent);
        }
        .sidebar-content {
          flex: 1;
          overflow-y: auto;
          overflow-x: hidden;
        }
        .empty-state {
          padding: 32px 16px;
          text-align: center;
          color: var(--text-tertiary);
          font-size: 13px;
        }
        .empty-state .hint {
          font-size: 12px;
          margin-top: 4px;
        }
        .sidebar-footer {
          padding: 8px;
          border-top: 1px solid var(--border);
        }
        .new-thread-btn {
          display: flex;
          align-items: center;
          gap: 6px;
          width: 100%;
          padding: 8px 14px;
          font-size: 13px;
          color: var(--text-secondary);
          border-radius: var(--radius-sm);
          transition: background 0.15s;
        }
        .new-thread-btn:hover {
          background: var(--bg-accent);
        }
        .new-thread-btn .plus {
          color: var(--primary);
          font-size: 16px;
        }
        .resize-handle {
          width: 4px;
          cursor: col-resize;
          background: var(--border);
          flex-shrink: 0;
          transition: background 0.15s;
        }
        .resize-handle:hover {
          background: var(--primary);
        }
      `}</style>
    </>
  );
}
