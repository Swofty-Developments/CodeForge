import { For, Show } from "solid-js";
import type { Project } from "../../types";
import { appStore } from "../../stores/app-store";
import { ThreadItem } from "./ThreadItem";
import { useSortable } from "@dnd-kit/solid/sortable";

function SortableThread(props: { thread: any; index: number; isUncategorized: boolean; groupColor: string | null }) {
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
    />
  );
}

export function ProjectGroup(props: { project: Project }) {
  const { store, setStore, newThread } = appStore;

  const isUncategorized = () => props.project.path === ".";

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
