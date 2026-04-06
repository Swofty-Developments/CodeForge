import { For, Show, createMemo } from "solid-js";
import type { Project } from "../../types";
import { appStore } from "../../stores/app-store";
import { ThreadItem } from "./ThreadItem";
import { useSortable } from "@dnd-kit/solid/sortable";

function SortableThread(props: { thread: any; index: number; isUncategorized: boolean; groupColor: string | null; prNumber?: number; hasWorktree?: boolean; staggerIndex?: number }) {
  const sortable = useSortable({
    get id() { return props.thread.id; },
    get index() { return props.index; },
    group: "sidebar-threads",
    get disabled() { return !props.isUncategorized; },
  });

  return (
    <ThreadItem
      thread={props.thread}
      isUncategorized={props.isUncategorized}
      groupColor={props.groupColor}
      sortableRef={sortable.ref}
      isDragging={typeof sortable.isDragging === 'function' ? sortable.isDragging() : !!sortable.isDragging}
      prNumber={props.prNumber}
      hasWorktree={props.hasWorktree}
      staggerIndex={props.staggerIndex}
    />
  );
}

export function ProjectGroup(props: { project: Project }) {
  const { store, setStore, newThread, loadProjectGitStatus } = appStore;

  const isUncategorized = () => props.project.path === ".";

  // Direction B: detect if active thread belongs to this project
  const isActiveProject = createMemo(() => {
    if (!store.activeTab) return false;
    return props.project.threads.some((t) => t.id === store.activeTab);
  });

  // Direction B: summary for collapsed state
  const collapsedSummary = createMemo(() => {
    const threads = props.project.threads;
    const generating = threads.filter(
      (t) => store.runStates[t.id] === "generating" || store.runStates[t.id] === "starting"
    ).length;
    if (generating > 0) return `${threads.length} threads · ${generating} active`;
    return `${threads.length} thread${threads.length !== 1 ? "s" : ""}`;
  });

  // Git detection — cached in store to avoid re-fetching on every mount
  const gitStatus = () => store.projectGitStatus[props.project.id] || "none";
  const prMap = () => store.projectPrMap[props.project.id] || {};

  // Load git status once, results cached in store across re-mounts
  if (props.project.path !== ".") {
    setTimeout(() => {
      loadProjectGitStatus(props.project.id, props.project.path, props.project.threads);
    }, 600);
  }

  function toggleCollapse() {
    setStore("projects", (projects) =>
      projects.map((p) => p.id === props.project.id ? { ...p, collapsed: !p.collapsed } : p)
    );
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    setStore("contextMenu", { type: "project", id: props.project.id, x: e.clientX, y: e.clientY });
  }

  return (
    <div
      class="pg"
      classList={{
        "pg--uncat": isUncategorized(),
        "pg--active": isActiveProject() && !isUncategorized(),
      }}
      style={isActiveProject() && props.project.color ? { "--pg-glow": props.project.color } as any : {}}
      data-project-id={props.project.id}
      onContextMenu={handleContextMenu}
    >
      <div class="pg-header">
        <button class="pg-toggle" onClick={toggleCollapse}>
          <svg
            class="pg-chevron"
            classList={{ "pg-chevron--open": !props.project.collapsed }}
            width="10" height="10" viewBox="0 0 24 24"
            fill="none" stroke="currentColor" stroke-width="2.5"
            stroke-linecap="round" stroke-linejoin="round"
          >
            <polyline points="9 18 15 12 9 6" />
          </svg>
          <Show when={!isUncategorized() && (gitStatus() === "github" || gitStatus() === "git")}>
            <Show when={gitStatus() === "github"} fallback={
              <svg class="pg-type-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M6 21V9a9 9 0 009 9" />
              </svg>
            }>
              <svg class="pg-type-icon" width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
              </svg>
            </Show>
          </Show>
          <span
            class="pg-name"
            style={props.project.color ? { color: props.project.color } : {}}
          >
            {props.project.name}
          </span>
          <span class="pg-count">{props.project.threads.length}</span>
          <Show when={props.project.collapsed && !isUncategorized()}>
            <span class="pg-summary">{collapsedSummary()}</span>
          </Show>
        </button>
        <button class="pg-add" onClick={(e) => { e.stopPropagation(); newThread(props.project.id); }} title="New thread">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
            <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
          </svg>
        </button>
      </div>

      <div class="pg-threads-wrap" classList={{ collapsed: !!props.project.collapsed }}>
        <div class="pg-threads">
          <For each={props.project.threads}>
            {(thread, idx) => (
              <SortableThread
                thread={thread}
                index={idx()}
                isUncategorized={isUncategorized()}
                groupColor={props.project.color}
                prNumber={prMap()[thread.id]}
                hasWorktree={!!store.worktrees[thread.id]}
                staggerIndex={idx()}
              />
            )}
          </For>
        </div>
      </div>

      <style>{`
        .pg {
          margin-bottom: var(--space-1);
          position: relative;
          transition: border-color 0.2s;
        }
        /* Direction B: glowing left border on active project */
        .pg--active {
          border-left: 2px solid var(--pg-glow, var(--primary));
          padding-left: 0;
          border-radius: 2px 0 0 2px;
          box-shadow: -2px 0 8px -2px color-mix(in srgb, var(--pg-glow, var(--primary)) 30%, transparent);
        }
        .pg--uncat {
          margin-top: var(--space-2);
          padding-top: var(--space-2);
          border-top: 1px solid var(--border);
        }
        .pg-header {
          display: flex;
          align-items: center;
          padding: 0 var(--space-1);
        }
        .pg-toggle {
          display: flex;
          align-items: center;
          gap: var(--space-2);
          flex: 1;
          padding: var(--space-1) var(--space-2);
          border-radius: var(--radius-sm);
          transition: background 0.12s;
          text-align: left;
        }
        .pg-toggle:hover { background: var(--bg-hover); }
        .pg-chevron {
          color: var(--text-tertiary);
          flex-shrink: 0;
          transition: transform 0.15s ease;
        }
        .pg-chevron--open { transform: rotate(90deg); }
        .pg-type-icon {
          color: var(--text-tertiary);
          flex-shrink: 0;
        }
        .pg-name {
          font-size: 11px;
          font-weight: 600;
          letter-spacing: 0.02em;
          color: var(--text-secondary);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .pg-count {
          font-size: 10px;
          font-family: var(--font-mono);
          color: var(--text-tertiary);
          flex-shrink: 0;
          min-width: 14px;
          text-align: center;
        }
        /* Direction B: collapsed summary */
        .pg-summary {
          font-size: 9px;
          font-family: var(--font-mono);
          color: var(--text-tertiary);
          opacity: 0.6;
          margin-left: var(--space-1);
          white-space: nowrap;
        }
        .pg-add {
          color: var(--text-tertiary);
          padding: var(--space-1);
          border-radius: var(--radius-sm);
          display: flex;
          align-items: center;
          transition: background 0.12s, color 0.12s;
          flex-shrink: 0;
        }
        .pg-add:hover { background: var(--bg-accent); color: var(--text-secondary); }
        .pg-threads-wrap {
          display: grid;
          grid-template-rows: 1fr;
          transition: grid-template-rows 200ms cubic-bezier(0.16, 1, 0.3, 1);
        }
        .pg-threads-wrap.collapsed {
          grid-template-rows: 0fr;
        }
        .pg-threads {
          overflow: hidden;
        }
      `}</style>
    </div>
  );
}
