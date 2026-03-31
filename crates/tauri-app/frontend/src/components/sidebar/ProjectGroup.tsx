import { For, Show, createSignal } from "solid-js";
import type { Project } from "../../types";
import { appStore } from "../../stores/app-store";
import { ThreadItem } from "./ThreadItem";
import { useSortable } from "@dnd-kit/solid/sortable";
import * as ipc from "../../ipc";

function SortableThread(props: { thread: any; index: number; isUncategorized: boolean; groupColor: string | null; prNumber?: number; hasWorktree?: boolean }) {
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
      isDragging={sortable.isDragging}
      prNumber={props.prNumber}
      hasWorktree={props.hasWorktree}
    />
  );
}

export function ProjectGroup(props: { project: Project }) {
  const { store, setStore, newThread } = appStore;

  const isUncategorized = () => props.project.path === ".";

  // Git detection — stored as local signals, NOT affecting the <For> loop.
  // These only affect the header icon and props passed to threads.
  const [gitStatus, setGitStatus] = createSignal<"none" | "git" | "github">("none");
  const [prMap, setPrMap] = createSignal<Record<string, number>>({});

  // Run git check ONCE via setTimeout to avoid disrupting initial render/click handlers
  if (props.project.path !== ".") {
    setTimeout(async () => {
      try {
        const isGh = await ipc.isGithubRepo(props.project.path);
        if (isGh) {
          setGitStatus("github");
        } else {
          try {
            await ipc.getChangedFiles(props.project.path);
            setGitStatus("git");
          } catch { /* not a git repo */ }
        }
      } catch { /* gh not installed or error */ }

      // Load PR associations if git
      if (gitStatus() !== "none") {
        const map: Record<string, number> = {};
        for (const t of props.project.threads) {
          try {
            const val = await ipc.getSetting(`pr:${t.id}`);
            if (val) map[t.id] = parseInt(val, 10);
          } catch {}
        }
        if (Object.keys(map).length > 0) setPrMap(map);
      }
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

  const color = () => props.project.color || "var(--text)";

  return (
    <div
      class="project-group"
      data-project-id={props.project.id}
      onContextMenu={handleContextMenu}
    >
      <div class="project-header">
        <button class="project-toggle" onClick={toggleCollapse}>
          <span class="collapse-icon">{props.project.collapsed ? "\u25B6" : "\u25BC"}</span>
          <span class="project-name" style={{ color: color() }}>{props.project.name.toUpperCase()}</span>
          {/* Git indicator — only in header, never affects thread list */}
          <Show when={gitStatus() === "github"}>
            <svg class="pg-git-icon" width="11" height="11" viewBox="0 0 24 24" fill="var(--text-tertiary)">
              <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
            </svg>
          </Show>
          <Show when={gitStatus() === "git"}>
            <svg class="pg-git-icon" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--text-tertiary)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M6 21V9a9 9 0 009 9" />
            </svg>
          </Show>
        </button>
        <button class="project-add" onClick={(e) => { e.stopPropagation(); newThread(props.project.id); }}>+</button>
      </div>

      <Show when={!props.project.collapsed}>
        <div class="project-threads">
          <For each={props.project.threads}>
            {(thread, idx) => (
              <SortableThread
                thread={thread}
                index={idx()}
                isUncategorized={isUncategorized()}
                groupColor={props.project.color}
                prNumber={prMap()[thread.id]}
                hasWorktree={!!store.worktrees[thread.id]}
              />
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}

if (!document.getElementById("project-group-styles")) {
  const s = document.createElement("style");
  s.id = "project-group-styles";
  s.textContent = `
    .project-group { margin-bottom: 2px; border-radius: var(--radius-sm); }
    .project-header { display: flex; align-items: center; padding: 0 4px; }
    .project-toggle {
      display: flex; align-items: center; gap: 6px; flex: 1;
      padding: 6px 8px; border-radius: var(--radius-sm);
      transition: background 0.15s; text-align: left;
    }
    .project-toggle:hover { background: var(--bg-muted); }
    .collapse-icon { font-size: 8px; color: var(--text-secondary); }
    .project-name { font-size: 10px; letter-spacing: 0.05em; }
    .pg-git-icon { margin-left: 2px; flex-shrink: 0; opacity: 0.6; }
    .project-add {
      font-size: 13px; color: var(--text-secondary); padding: 4px 8px;
      border-radius: var(--radius-sm); transition: background 0.15s;
    }
    .project-add:hover { background: var(--bg-accent); }
    @keyframes threads-expand {
      from { opacity: 0; max-height: 0; transform: translateY(-4px); }
      to { opacity: 1; max-height: 2000px; transform: translateY(0); }
    }
    .project-threads {
      animation: threads-expand 250ms cubic-bezier(0.16, 1, 0.3, 1) both;
      overflow: hidden;
    }
  `;
  document.head.appendChild(s);
}
