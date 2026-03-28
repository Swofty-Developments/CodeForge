import { For, Show } from "solid-js";
import type { Project } from "../../types";
import { appStore } from "../../stores/app-store";
import { ThreadItem } from "./ThreadItem";

export function ProjectGroup(props: { project: Project }) {
  const { store, setStore, newThread } = appStore;

  const isUncategorized = () => props.project.path === ".";
  const isDragTarget = () =>
    store.draggingSidebarThread !== null && !isUncategorized();

  function toggleCollapse() {
    setStore(
      "projects",
      (p) => p.id === props.project.id,
      "collapsed",
      (c) => !c
    );
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    const threadId = e.dataTransfer?.getData("text/thread-id");
    if (threadId) {
      import("../../ipc").then(({ moveThreadToProject }) => {
        moveThreadToProject(threadId, props.project.id);
        // Move in local state
        setStore("projects", (projects) => {
          const updated = projects.map((p) => ({
            ...p,
            threads: p.threads.filter((t) => t.id !== threadId),
          }));
          const targetIdx = updated.findIndex((p) => p.id === props.project.id);
          const thread = projects
            .flatMap((p) => p.threads)
            .find((t) => t.id === threadId);
          if (targetIdx !== -1 && thread) {
            updated[targetIdx] = {
              ...updated[targetIdx],
              threads: [...updated[targetIdx].threads, thread],
            };
          }
          return updated;
        });
      });
    }
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    setStore("contextMenu", {
      type: "project",
      id: props.project.id,
      x: e.clientX,
      y: e.clientY,
    });
  }

  const color = () => props.project.color || "var(--text-tertiary)";

  return (
    <div
      class="project-group"
      onContextMenu={handleContextMenu}
      onDragOver={(e) => { e.preventDefault(); e.dataTransfer!.dropEffect = "move"; }}
      onDrop={handleDrop}
      classList={{ "drop-target": isDragTarget() }}
    >
      <div class="project-header">
        <button class="project-toggle" onClick={toggleCollapse}>
          <span class="collapse-icon">
            {props.project.collapsed ? "\u25B6" : "\u25BC"}
          </span>
          <span class="project-name" style={{ color: color() }}>
            {props.project.name.toUpperCase()}
          </span>
        </button>
        <button
          class="project-add"
          onClick={(e) => {
            e.stopPropagation();
            newThread(props.project.id);
          }}
          title="New thread in group"
        >
          +
        </button>
      </div>

      <Show when={!props.project.collapsed}>
        <For each={props.project.threads}>
          {(thread) => (
            <ThreadItem thread={thread} isUncategorized={isUncategorized()} />
          )}
        </For>
      </Show>
    </div>
  );
}

// Inject styles once
if (!document.getElementById("project-group-styles")) {
  const style = document.createElement("style");
  style.id = "project-group-styles";
  style.textContent = `
    .project-group { margin-bottom: 2px; }
    .project-group.drop-target {
      outline: 2px solid var(--primary);
      outline-offset: -2px;
      border-radius: var(--radius-sm);
    }
    .project-header {
      display: flex;
      align-items: center;
      padding: 0 4px;
    }
    .project-toggle {
      display: flex;
      align-items: center;
      gap: 6px;
      flex: 1;
      padding: 6px 8px;
      border-radius: var(--radius-sm);
      transition: background 0.15s;
      text-align: left;
    }
    .project-toggle:hover { background: var(--bg-muted); }
    .collapse-icon { font-size: 8px; color: var(--text-tertiary); }
    .project-name { font-size: 10px; letter-spacing: 0.05em; }
    .project-add {
      font-size: 13px;
      color: var(--text-tertiary);
      padding: 4px 8px;
      border-radius: var(--radius-sm);
      transition: background 0.15s;
    }
    .project-add:hover { background: var(--bg-accent); }
  `;
  document.head.appendChild(style);
}
