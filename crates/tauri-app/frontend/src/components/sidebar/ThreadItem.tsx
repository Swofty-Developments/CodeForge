import { Show, createSignal, createEffect, on } from "solid-js";
import type { Thread } from "../../types";
import { appStore } from "../../stores/app-store";

export function ThreadItem(props: {
  thread: Thread;
  isUncategorized: boolean;
  groupColor: string | null;
  sortableRef?: (el: HTMLElement) => void;
  isDragging?: boolean;
  prNumber?: number;
  hasWorktree?: boolean;
  staggerIndex?: number;
}) {
  const { store, setStore, selectThread } = appStore;

  const isActive = () => store.activeTab === props.thread.id;
  const isRenaming = () => store.renamingThread?.id === props.thread.id;
  const prStatus = () => store.threadPrStatus[props.thread.id] || null;
  const [showTooltip, setShowTooltip] = createSignal(false);
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;

  // Direction A: completion pulse — detect when a thread finishes generating
  const [justCompleted, setJustCompleted] = createSignal(false);
  let prevStatus: string | undefined;
  createEffect(on(
    () => store.sessionStatuses[props.thread.id],
    (status) => {
      if (prevStatus === "generating" && status === "ready" && !isActive()) {
        setJustCompleted(true);
        setTimeout(() => setJustCompleted(false), 1500);
      }
      prevStatus = status;
    }
  ));

  const projectName = () => {
    const project = store.projects.find((p) => p.threads.some((t) => t.id === props.thread.id));
    return project?.name || "";
  };

  const worktreeBranch = () => {
    const wt = store.worktrees[props.thread.id];
    return wt?.active ? wt.branch : null;
  };

  const lastMessagePreview = () => {
    const msgs = store.threadMessages[props.thread.id];
    if (!msgs || msgs.length === 0) return null;
    const last = msgs[msgs.length - 1];
    const text = last.content?.slice(0, 50);
    return text ? (text.length >= 50 ? text + "..." : text) : null;
  };

  const statusColor = () => {
    const status = store.sessionStatuses[props.thread.id];
    if (!status || status === "idle") return null;
    if (status === "ready") return "var(--green)";
    if (status === "generating" || status === "starting") return "var(--sky)";
    if (status === "error") return "var(--red)";
    return null;
  };

  const isGenerating = () => {
    const status = store.sessionStatuses[props.thread.id];
    return status === "generating" || status === "starting";
  };

  function handleMouseEnter() {
    hoverTimer = setTimeout(() => setShowTooltip(true), 400);
  }

  function handleMouseLeave() {
    clearTimeout(hoverTimer);
    setShowTooltip(false);
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setStore("contextMenu", { type: "thread", id: props.thread.id, x: e.clientX, y: e.clientY });
  }

  function handleRenameSubmit(e: Event) {
    e.preventDefault();
    const text = store.renamingThread?.text?.trim();
    if (text) {
      import("../../ipc").then(({ renameThread }) => {
        renameThread(props.thread.id, text);
        setStore("projects", (projects) =>
          projects.map((p) => ({
            ...p,
            threads: p.threads.map((t) => t.id === props.thread.id ? { ...t, title: text } : t),
          }))
        );
      });
    }
    setStore("renamingThread", null);
  }

  return (
    <>
      <Show
        when={!isRenaming()}
        fallback={
          <form class="ti-rename" onSubmit={handleRenameSubmit}>
            <input
              value={store.renamingThread?.text || ""}
              onInput={(e) => setStore("renamingThread", "text", e.currentTarget.value)}
              onBlur={handleRenameSubmit}
              autofocus
            />
          </form>
        }
      >
        <div
          ref={props.sortableRef}
          class="ti"
          classList={{
            "ti--active": isActive(),
            "ti--dragging": !!props.isDragging,
            "ti--draggable": props.isUncategorized,
            "ti--generating": isGenerating(),
            "ti--completed": justCompleted(),
          }}
          style={{
            "animation-delay": props.staggerIndex != null ? `${props.staggerIndex * 30}ms` : undefined,
          }}
          onClick={() => selectThread(props.thread.id)}
          onContextMenu={handleContextMenu}
          onDblClick={() => setStore("renamingThread", { id: props.thread.id, text: props.thread.title })}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
          {/* Status indicator */}
          <Show when={statusColor()}>
            <span
              class="ti-dot"
              classList={{ "ti-dot--pulse": isGenerating() }}
              style={{ background: statusColor()! }}
            />
          </Show>

          {/* Title + badges */}
          <div class="ti-content">
            <span class="ti-title">{props.thread.title}</span>
            <Show when={props.prNumber || props.hasWorktree || prStatus()}>
              <div class="ti-badges">
                <Show when={props.prNumber}>
                  <span class="ti-badge ti-badge--pr">#{props.prNumber}</span>
                </Show>
                {/* CI status badge */}
                <Show when={prStatus()?.ci_status && prStatus()!.ci_status !== "none"}>
                  <span class="ti-badge" classList={{
                    "ti-badge--ci-pass": prStatus()!.ci_status === "success",
                    "ti-badge--ci-fail": prStatus()!.ci_status === "failure",
                    "ti-badge--ci-pending": prStatus()!.ci_status === "pending",
                  }}>
                    {prStatus()!.ci_status === "success" ? "✓ CI" :
                     prStatus()!.ci_status === "failure" ? "✗ CI" : "⏳ CI"}
                  </span>
                </Show>
                {/* Review status badge */}
                <Show when={prStatus()?.review_status && prStatus()!.review_status !== "none"}>
                  <span class="ti-badge" classList={{
                    "ti-badge--review-approved": prStatus()!.review_status === "approved",
                    "ti-badge--review-changes": prStatus()!.review_status === "changes_requested",
                  }}>
                    {prStatus()!.review_status === "approved" ? "✓ Approved" : "⚠ Changes"}
                  </span>
                </Show>
                <Show when={props.hasWorktree && !props.prNumber}>
                  <span class="ti-badge ti-badge--wt">
                    <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                      <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M6 21V9a9 9 0 009 9" />
                    </svg>
                    branch
                  </span>
                </Show>
              </div>
            </Show>
          </div>

          {/* Hover actions */}
          <div class="ti-actions">
            <button
              class="ti-act"
              onClick={(e) => { e.stopPropagation(); setStore("renamingThread", { id: props.thread.id, text: props.thread.title }); }}
              title="Rename"
            >
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M17 3a2.83 2.83 0 114 4L7.5 20.5 2 22l1.5-5.5L17 3z" />
              </svg>
            </button>
            <button
              class="ti-act ti-act--del"
              onClick={(e) => {
                e.stopPropagation();
                import("../../ipc").then(({ deleteThread }) => {
                  deleteThread(props.thread.id);
                  setStore("projects", (projects) =>
                    projects.map((p) => ({ ...p, threads: p.threads.filter((t) => t.id !== props.thread.id) }))
                  );
                  setStore("openTabs", (tabs) => tabs.filter((t) => t !== props.thread.id));
                  if (store.activeTab === props.thread.id) {
                    setStore("activeTab", store.openTabs.filter((t) => t !== props.thread.id).pop() || null);
                  }
                });
              }}
              title="Delete"
            >
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>

          {/* Hover tooltip */}
          <Show when={showTooltip()}>
            <div class="ti-tooltip">
              <div class="ti-tooltip-title">{props.thread.title}</div>
              <Show when={projectName()}>
                <div class="ti-tooltip-row">
                  <span class="ti-tooltip-label">Project</span>
                  <span>{projectName()}</span>
                </div>
              </Show>
              <Show when={worktreeBranch()}>
                <div class="ti-tooltip-row">
                  <span class="ti-tooltip-label">Branch</span>
                  <span>{worktreeBranch()}</span>
                </div>
              </Show>
              <Show when={props.prNumber}>
                <div class="ti-tooltip-row">
                  <span class="ti-tooltip-label">PR</span>
                  <span>#{props.prNumber}</span>
                </div>
              </Show>
              <Show when={lastMessagePreview()}>
                <div class="ti-tooltip-preview">{lastMessagePreview()}</div>
              </Show>
            </div>
          </Show>
        </div>
      </Show>

      <style>{`
        .ti {
          display: flex;
          align-items: center;
          gap: var(--space-2);
          padding: 6px var(--space-3);
          margin: 1px var(--space-1);
          border-radius: var(--radius-sm);
          cursor: pointer;
          transition: background 0.15s, box-shadow 0.2s, transform 0.15s, filter 0.15s;
          position: relative;
          animation: ti-stagger-in 0.2s ease-out both;
        }
        @keyframes ti-stagger-in {
          from { opacity: 0; transform: translateX(-6px); }
          to { opacity: 1; transform: translateX(0); }
        }
        .ti[aria-disabled="true"] {
          opacity: 1;
          cursor: pointer;
        }
        .ti:hover { background: var(--bg-hover); }

        /* Direction C: Active thread lifts with depth */
        .ti--active {
          background: var(--bg-accent);
          box-shadow: 0 1px 6px rgba(0, 0, 0, 0.15);
          z-index: 1;
        }
        .ti--active::before {
          content: "";
          position: absolute;
          left: 0;
          top: 6px;
          bottom: 6px;
          width: 2px;
          border-radius: 1px;
          background: var(--primary);
        }

        /* Direction A: Generating thread shimmer */
        .ti--generating::after {
          content: "";
          position: absolute;
          left: 0;
          top: 0;
          bottom: 0;
          width: 2px;
          border-radius: 1px;
          background: linear-gradient(
            to bottom,
            transparent 0%,
            var(--sky) 50%,
            transparent 100%
          );
          background-size: 100% 200%;
          animation: ti-shimmer 1.2s ease-in-out infinite;
        }
        @keyframes ti-shimmer {
          from { background-position: 0 0; }
          to { background-position: 0 -200%; }
        }

        /* Direction A: Completion pulse */
        .ti--completed {
          animation: ti-complete-pulse 1.5s ease-out;
        }
        @keyframes ti-complete-pulse {
          0% { box-shadow: 0 0 0 0 rgba(76, 214, 148, 0.35); }
          30% { box-shadow: 0 0 8px 1px rgba(76, 214, 148, 0.25); background: rgba(76, 214, 148, 0.06); }
          100% { box-shadow: 0 0 0 0 transparent; background: transparent; }
        }

        .ti--dragging { opacity: 0.3; will-change: transform; }
        .ti--draggable { cursor: grab; touch-action: none; }
        .ti--draggable:active { cursor: grabbing; will-change: transform; }

        @media (prefers-reduced-motion: reduce) {
          .ti { animation: none !important; }
          .ti--generating::after { animation: none; }
          .ti--completed { animation: none; }
        }

        /* Status dot */
        .ti-dot {
          width: 5px;
          height: 5px;
          border-radius: 50%;
          flex-shrink: 0;
        }
        .ti-dot--pulse {
          animation: ti-pulse 1.5s ease-in-out infinite;
        }
        @keyframes ti-pulse {
          0%, 100% { opacity: 1; transform: scale(1); }
          50% { opacity: 0.4; transform: scale(0.7); }
        }

        /* Content area */
        .ti-content {
          flex: 1;
          min-width: 0;
          display: flex;
          flex-direction: column;
          gap: 2px;
        }
        .ti-title {
          font-size: 12px;
          color: var(--text-secondary);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
          line-height: 1.3;
        }
        .ti--active .ti-title {
          color: var(--text);
          font-weight: 500;
        }
        .ti:hover .ti-title { color: var(--text); }

        /* Badges row */
        .ti-badges {
          display: flex;
          gap: var(--space-1);
        }
        .ti-badge {
          font-size: 9px;
          font-weight: 500;
          font-family: var(--font-mono);
          padding: 0 var(--space-1);
          border-radius: 3px;
          white-space: nowrap;
          display: flex;
          align-items: center;
          gap: 3px;
          line-height: 1.5;
        }
        .ti-badge--pr {
          color: var(--primary);
          background: rgba(107, 124, 255, 0.1);
        }
        .ti-badge--wt {
          color: var(--text-tertiary);
          background: var(--bg-muted);
        }
        .ti-badge--ci-pass {
          color: var(--green);
          background: rgba(76, 214, 148, 0.1);
        }
        .ti-badge--ci-fail {
          color: var(--red);
          background: rgba(242, 95, 103, 0.1);
        }
        .ti-badge--ci-pending {
          color: var(--amber);
          background: rgba(240, 184, 64, 0.1);
        }
        .ti-badge--review-approved {
          color: var(--green);
          background: rgba(76, 214, 148, 0.1);
        }
        .ti-badge--review-changes {
          color: var(--amber);
          background: rgba(240, 184, 64, 0.1);
        }

        /* Hover actions */
        .ti-actions {
          display: flex;
          align-items: center;
          gap: 2px;
          opacity: 0;
          transition: opacity 0.1s;
          flex-shrink: 0;
        }
        .ti:hover .ti-actions { opacity: 1; }
        .ti--active .ti-actions { opacity: 1; }
        .ti-act {
          color: var(--text-tertiary);
          padding: 3px;
          border-radius: 3px;
          display: flex;
          align-items: center;
          transition: background 0.1s, color 0.1s;
        }
        .ti-act:hover { background: var(--bg-accent); color: var(--text-secondary); }
        .ti-act--del:hover { color: var(--red); }

        /* Rename */
        .ti-rename { padding: 2px var(--space-3); margin: 1px var(--space-1); }
        .ti-rename input { width: 100%; font-size: 12px; }

        /* Tooltip */
        .ti-tooltip {
          position: absolute;
          left: calc(100% + var(--space-2));
          top: 50%;
          transform: translateY(-50%);
          background: var(--bg-card);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-sm);
          padding: var(--space-2) var(--space-3);
          font-size: 11px;
          color: var(--text-secondary);
          z-index: 200;
          box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
          min-width: 160px;
          max-width: 240px;
          pointer-events: none;
          animation: ti-tip-in 0.12s ease-out;
        }
        @keyframes ti-tip-in {
          from { opacity: 0; transform: translateY(-50%) translateX(-4px); }
          to { opacity: 1; transform: translateY(-50%) translateX(0); }
        }
        .ti-tooltip-title {
          font-weight: 600;
          font-size: 12px;
          color: var(--text);
          margin-bottom: var(--space-1);
          word-break: break-word;
        }
        .ti-tooltip-row {
          display: flex;
          justify-content: space-between;
          gap: var(--space-2);
          padding: 1px 0;
          font-family: var(--font-mono);
          font-size: 10px;
        }
        .ti-tooltip-label { color: var(--text-tertiary); }
        .ti-tooltip-preview {
          margin-top: var(--space-1);
          padding-top: var(--space-1);
          border-top: 1px solid var(--border);
          font-size: 10px;
          color: var(--text-tertiary);
          font-style: italic;
          word-break: break-word;
        }
      `}</style>
    </>
  );
}
