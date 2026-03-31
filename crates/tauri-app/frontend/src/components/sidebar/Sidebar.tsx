import { For, Show } from "solid-js";
import { DragDropProvider } from "@dnd-kit/solid";
import { appStore } from "../../stores/app-store";
import { ProjectGroup } from "./ProjectGroup";
import * as ipc from "../../ipc";

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
          <div style={{ display: "flex", gap: "2px" }}>
            <button
              class="icon-btn"
              onClick={() => setStore("themeOpen", true)}
              title="Themes"
            >
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.93 0 1.5-.67 1.5-1.5 0-.38-.15-.74-.42-1.02-.27-.28-.42-.64-.42-1.02 0-.83.67-1.5 1.5-1.5H16c3.31 0 6-2.69 6-6 0-5.17-4.5-9-10-9z" />
                <circle cx="7.5" cy="11.5" r="1.5" fill="currentColor" />
                <circle cx="12" cy="7.5" r="1.5" fill="currentColor" />
                <circle cx="16.5" cy="11.5" r="1.5" fill="currentColor" />
              </svg>
            </button>
            <button
              class="icon-btn"
              onClick={() => setStore("settingsOpen", true)}
              title="Settings"
            >
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z" />
              </svg>
            </button>
          </div>
        </div>

        <div class="sidebar-content">
          <DragDropProvider
            onDragEnd={(event) => {
              const dragId = event.operation?.source?.id;
              const targetEl = document.elementFromPoint(
                event.operation?.position?.current?.x ?? 0,
                event.operation?.position?.current?.y ?? 0
              );
              let el = targetEl;
              while (el && !el.getAttribute?.("data-project-id")) {
                el = el.parentElement;
              }
              const targetProjectId = el?.getAttribute?.("data-project-id");
              if (dragId && targetProjectId) {
                const threadId = String(dragId);
                const srcProject = store.projects.find((p) =>
                  p.threads.some((t) => t.id === threadId)
                );
                if (srcProject && srcProject.path === "." && targetProjectId !== srcProject.id) {
                  ipc.moveThreadToProject(threadId, targetProjectId);
                  setStore("projects", (projects) => {
                    const thread = projects
                      .flatMap((p) => p.threads)
                      .find((t) => t.id === threadId);
                    if (!thread) return projects;
                    return projects.map((p) => ({
                      ...p,
                      threads:
                        p.id === targetProjectId
                          ? [...p.threads.filter((t) => t.id !== threadId), thread]
                          : p.threads.filter((t) => t.id !== threadId),
                    }));
                  });
                }
              }
            }}
          >
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
          </DragDropProvider>
        </div>

        <div class="sidebar-footer">
          <div class="sidebar-actions-grid">
            <button class="sidebar-action" onClick={() => appStore.openVirtualTab("__mcp__")} title="MCP Servers">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="2" width="20" height="8" rx="2" /><rect x="2" y="14" width="20" height="8" rx="2" /><circle cx="6" cy="6" r="1" fill="currentColor" /><circle cx="6" cy="18" r="1" fill="currentColor" />
              </svg>
              MCP
            </button>
            <button class="sidebar-action" onClick={() => appStore.openVirtualTab("__themes__")} title="Themes">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="13.5" cy="6.5" r="2.5" /><circle cx="17.5" cy="10.5" r="2.5" /><circle cx="8.5" cy="7.5" r="2.5" /><circle cx="6.5" cy="12.5" r="2.5" /><path d="M12 22C6.5 22 2 17.5 2 12S6.5 2 12 2s10 4.5 10 10-1.5 4-3 4h-1.7c-.8 0-1.3.8-.9 1.5.6 1.1 1 2.2 1 3.5 0 1.5-.5 2-1.4 2z" />
              </svg>
              Themes
            </button>
            <button class="sidebar-action" onClick={() => setStore("searchOpen", true)} title="Search (Cmd+Shift+F)">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              Search
            </button>
            <button class="sidebar-action" onClick={() => setStore("usageDashboardOpen", true)} title="Usage (Cmd+Shift+U)">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                <line x1="18" y1="20" x2="18" y2="10" /><line x1="12" y1="20" x2="12" y2="4" /><line x1="6" y1="20" x2="6" y2="14" />
              </svg>
              Usage
            </button>
          </div>
          <button class="new-thread-btn" onClick={() => newThread()}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
              <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            New Thread
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
          padding: 16px 16px 12px;
          justify-content: space-between;
        }
        .sidebar-title {
          font-size: 14px;
          font-weight: 700;
          letter-spacing: -0.3px;
          background: linear-gradient(135deg, var(--text) 40%, var(--primary));
          -webkit-background-clip: text;
          -webkit-text-fill-color: transparent;
          background-clip: text;
        }
        .icon-btn {
          color: var(--text-tertiary);
          padding: 5px;
          border-radius: var(--radius-sm);
          transition: background 0.15s, color 0.15s;
          display: flex;
          align-items: center;
        }
        .icon-btn:hover {
          background: var(--bg-accent);
          color: var(--text-secondary);
        }
        .sidebar-content {
          flex: 1;
          overflow-y: auto;
          overflow-x: hidden;
          padding: 0 4px;
        }
        .empty-state {
          padding: 40px 16px;
          text-align: center;
          color: var(--text-tertiary);
          font-size: 13px;
        }
        .empty-state .hint {
          font-size: 12px;
          margin-top: 6px;
          opacity: 0.6;
        }
        .sidebar-footer {
          padding: 10px 10px 12px;
          border-top: 1px solid var(--border);
          display: flex;
          flex-direction: column;
          gap: 6px;
        }
        .new-thread-btn {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 8px;
          width: 100%;
          padding: 8px 12px;
          font-size: 13px;
          font-weight: 500;
          color: var(--text-secondary);
          border-radius: var(--radius-sm);
          transition: all 0.15s ease;
        }
        .new-thread-btn svg { color: var(--primary); flex-shrink: 0; }
        .new-thread-btn:hover {
          background: rgba(107, 124, 255, 0.08);
          color: var(--text);
        }
        .sidebar-actions-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 4px;
        }
        .sidebar-actions {
          display: flex;
          gap: 4px;
        }
        .sidebar-action {
          flex: 1;
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 5px;
          padding: 6px 8px;
          font-size: 11px;
          font-weight: 500;
          color: var(--text-tertiary);
          border-radius: var(--radius-sm);
          transition: all 0.15s ease;
        }
        .sidebar-action:hover {
          background: var(--bg-accent);
          color: var(--text-secondary);
        }
        .resize-handle {
          width: 3px;
          cursor: col-resize;
          background: transparent;
          flex-shrink: 0;
          transition: background 0.15s;
          position: relative;
        }
        .resize-handle::after {
          content: "";
          position: absolute;
          inset: 0;
          background: var(--border);
          transition: background 0.15s;
        }
        .resize-handle:hover::after {
          background: var(--primary);
        }
      `}</style>
    </>
  );
}
