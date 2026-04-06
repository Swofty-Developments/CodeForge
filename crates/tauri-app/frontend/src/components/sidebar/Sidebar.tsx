import { For, Show } from "solid-js";
import { DragDropProvider } from "@dnd-kit/solid";
import { appStore } from "../../stores/app-store";
import { ProjectGroup } from "./ProjectGroup";
import * as ipc from "../../ipc";

export function Sidebar() {
  const { store, setStore, newThread, addProject } = appStore;

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
      appStore.persistState();
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

        <div class="sidebar-dock">
          <button class="dock-btn" onClick={() => appStore.openVirtualTab("__mcp__")} title="MCP Servers">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="2" y="2" width="20" height="8" rx="2" /><rect x="2" y="14" width="20" height="8" rx="2" /><circle cx="6" cy="6" r="1" fill="currentColor" /><circle cx="6" cy="18" r="1" fill="currentColor" />
            </svg>
            <span>MCP</span>
          </button>
          <button class="dock-btn" onClick={() => appStore.openVirtualTab("__themes__")} title="Themes">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="13.5" cy="6.5" r="2.5" /><circle cx="17.5" cy="10.5" r="2.5" /><circle cx="8.5" cy="7.5" r="2.5" /><circle cx="6.5" cy="12.5" r="2.5" /><path d="M12 22C6.5 22 2 17.5 2 12S6.5 2 12 2s10 4.5 10 10-1.5 4-3 4h-1.7c-.8 0-1.3.8-.9 1.5.6 1.1 1 2.2 1 3.5 0 1.5-.5 2-1.4 2z" />
            </svg>
            <span>Themes</span>
          </button>
          <button class="dock-btn" onClick={() => appStore.openVirtualTab("__search__")} title="Search (Cmd+Shift+F)">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            <span>Search</span>
          </button>
          <button class="dock-btn" onClick={() => appStore.openVirtualTab("__skills__")} title="Skills & Plugins">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z" />
            </svg>
            <span>Skills</span>
          </button>
          <button class="dock-btn" onClick={() => setStore("usageDashboardOpen", true)} title="Usage (Cmd+Shift+U)">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
              <line x1="18" y1="20" x2="18" y2="10" /><line x1="12" y1="20" x2="12" y2="4" /><line x1="6" y1="20" x2="6" y2="14" />
            </svg>
            <span>Usage</span>
          </button>
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
                  const srcIdx = store.projects.findIndex((p) => p.id === srcProject.id);
                  const destIdx = store.projects.findIndex((p) => p.id === targetProjectId);
                  const thread = srcProject.threads.find((t) => t.id === threadId);
                  if (thread && srcIdx !== -1 && destIdx !== -1) {
                    setStore("projects", srcIdx, "threads", (threads: any[]) =>
                      threads.filter((t) => t.id !== threadId)
                    );
                    setStore("projects", destIdx, "threads", (threads: any[]) =>
                      [...threads, thread]
                    );
                  }
                }
              }
            }}
          >
            <Show
              when={store.projects.length > 0}
              fallback={
                <div class="empty-state">
                  <svg class="empty-state-icon" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" />
                    <line x1="9" y1="9" x2="15" y2="9" />
                    <line x1="12" y1="6" x2="12" y2="12" />
                  </svg>
                  <p class="empty-state-title">Start a conversation</p>
                  <p class="empty-state-desc">Create a thread to begin working with AI on your code</p>
                  <button class="empty-state-cta" onClick={() => newThread()}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                      <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
                    </svg>
                    New Thread
                  </button>
                  <span class="empty-state-hint">or <kbd>&#8984;T</kbd></span>
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
          <button class="new-thread-btn" onClick={() => newThread()}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
              <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            New Thread
          </button>
          <button class="add-project-btn" onClick={() => addProject()}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
              <line x1="12" y1="11" x2="12" y2="17" /><line x1="9" y1="14" x2="15" y2="14" />
            </svg>
            Add Project
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
          padding: var(--space-4) var(--space-4) var(--space-3);
          justify-content: space-between;
        }
        .sidebar-title {
          font-size: 14px;
          font-weight: 700;
          letter-spacing: -0.3px;
          color: var(--text);
        }
        .icon-btn {
          color: var(--text-tertiary);
          padding: var(--space-1);
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
          padding: var(--space-2) var(--space-1) 0;
        }
        .empty-state {
          padding: var(--space-10) var(--space-4);
          text-align: center;
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: var(--space-2);
        }
        .empty-state-icon {
          color: var(--text-tertiary);
          opacity: 0.5;
          margin-bottom: var(--space-1);
        }
        .empty-state-title {
          font-size: 14px;
          font-weight: 600;
          color: var(--text);
        }
        .empty-state-desc {
          font-size: 12px;
          color: var(--text-tertiary);
          line-height: 1.4;
          max-width: 180px;
        }
        .empty-state-cta {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: var(--space-2);
          padding: var(--space-2) var(--space-4);
          margin-top: var(--space-2);
          font-size: 13px;
          font-weight: 600;
          color: #fff;
          background: var(--primary);
          border-radius: var(--radius-sm);
          transition: filter 0.15s;
        }
        .empty-state-cta:hover { filter: brightness(1.15); }
        .empty-state-hint {
          font-size: 11px;
          color: var(--text-tertiary);
          opacity: 0.6;
        }
        .empty-state-hint kbd {
          font-family: var(--font-body);
          font-size: 10px;
          background: var(--bg-accent);
          border: 1px solid var(--border);
          border-radius: 3px;
          padding: 1px 4px;
        }
        .sidebar-footer {
          padding: var(--space-3);
          border-top: 1px solid var(--border);
          display: flex;
          flex-direction: column;
          gap: var(--space-2);
        }
        .add-project-btn {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: var(--space-2);
          width: 100%;
          padding: var(--space-2) var(--space-3);
          font-size: 13px;
          font-weight: 600;
          color: #fff;
          background: var(--primary);
          border-radius: var(--radius-sm);
          transition: all 0.15s ease;
        }
        .add-project-btn svg { color: #fff; flex-shrink: 0; }
        .add-project-btn:hover {
          filter: brightness(1.15);
        }
        .new-thread-btn {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: var(--space-2);
          width: 100%;
          padding: var(--space-2) var(--space-3);
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
        .sidebar-dock {
          display: flex;
          justify-content: space-between;
          padding: var(--space-1);
          margin: 0 var(--space-2);
          background: var(--bg-muted);
          border-radius: var(--radius-md);
          flex-shrink: 0;
        }
        .dock-btn {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 2px;
          padding: 5px var(--space-1);
          border-radius: var(--radius-sm);
          color: var(--text-tertiary);
          font-size: 10px;
          font-weight: 500;
          letter-spacing: 0.01em;
          flex: 1;
          transition: all 0.12s;
          position: relative;
          overflow: hidden;
        }
        /* Direction C: upward fill animation on hover */
        .dock-btn::before {
          content: "";
          position: absolute;
          inset: 0;
          background: var(--bg-accent);
          transform: translateY(100%);
          transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
          border-radius: inherit;
          z-index: 0;
        }
        .dock-btn:hover::before {
          transform: translateY(0);
        }
        .dock-btn > * {
          position: relative;
          z-index: 1;
        }
        .dock-btn:hover {
          color: var(--text-secondary);
        }
        .dock-btn svg {
          flex-shrink: 0;
          transition: color 0.12s;
        }
        .dock-btn:hover svg {
          color: var(--primary);
        }
        .resize-handle {
          width: 5px;
          cursor: col-resize;
          background: transparent;
          flex-shrink: 0;
          transition: background 0.15s;
          position: relative;
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .resize-handle::after {
          content: "";
          position: absolute;
          left: 2px;
          top: 0;
          bottom: 0;
          width: 1px;
          background: var(--border);
          transition: background 0.15s, box-shadow 0.15s;
        }
        .resize-handle:hover::after {
          background: var(--primary);
          box-shadow: 0 0 6px var(--primary-glow);
        }
        /* Grip dots — appear on hover */
        .resize-handle::before {
          content: "· · ·";
          position: absolute;
          left: 50%;
          top: 50%;
          transform: translate(-50%, -50%) rotate(90deg);
          font-size: 8px;
          color: var(--text-tertiary);
          letter-spacing: 2px;
          opacity: 0;
          transition: opacity 0.15s;
          pointer-events: none;
        }
        .resize-handle:hover::before {
          opacity: 0.5;
        }
      `}</style>
    </>
  );
}
