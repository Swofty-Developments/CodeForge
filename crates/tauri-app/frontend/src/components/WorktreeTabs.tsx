/* Worktree tab strip — one tab per open repo context (base first, then
 * worktrees). Each tab shows the branch, a dirty dot, and ahead/behind counts
 * from list_worktrees (refreshed on focus). The active tab is Zed-styled and
 * merges into the content surface. "+" opens the WorktreeSwitcher. Closing a
 * tab only closes the CONTEXT (C3) — it never removes the worktree from disk,
 * so it needs no confirm; removal is an explicit switcher action. */

import { For, Show, createMemo, onCleanup, onMount } from "solid-js";
import { appStore } from "../stores/app-store";
import type { RepoContext, Worktree } from "../types";
import { samePath } from "../stores/path";
import { WorktreeSwitcher } from "./worktree/WorktreeSwitcher";
import { hueForSlug } from "./graph/colors";

/** A context's project identity. `project` is filled by the backend (base-repo
 *  basename); a missing one (older repo:changed emit) falls to the context's own
 *  name — a named rule so the tab still groups with itself. */
const projectOf = (ctx: RepoContext): string => ctx.state.project ?? ctx.state.name;

/** Up to two initials: first letters of the first two words ("atomix-web" → AW),
 *  or the first two characters of a single-word name ("atomix" → AT). */
export function projectInitials(project: string): string {
  const parts = project.split(/[-_\s.]+/).filter(Boolean);
  const raw = parts.length >= 2 ? `${parts[0][0]}${parts[1][0]}` : project.slice(0, 2);
  return raw.toUpperCase();
}

export function WorktreeTabs() {
  const { store } = appStore;
  let newBtnRef: HTMLButtonElement | undefined;

  // Grouped by project (alphabetical), base first within a project, then open order.
  const ordered = createMemo(() =>
    [...store.contexts].sort((a, b) => {
      const pa = projectOf(a);
      const pb = projectOf(b);
      if (pa !== pb) return pa.localeCompare(pb, undefined, { sensitivity: "base" });
      return a.isBase === b.isBase ? 0 : a.isBase ? -1 : 1;
    }),
  );

  // Badges appear only when the open tabs span more than one project.
  const multiProject = createMemo(() => new Set(store.contexts.map(projectOf)).size > 1);

  const metaFor = (ctx: RepoContext): Worktree | undefined =>
    store.worktrees.find((w) => samePath(String(w.path), ctx.state.path));

  const label = (ctx: RepoContext): string =>
    metaFor(ctx)?.branch ?? ctx.state.branch ?? ctx.state.name;

  function activate(ctx: RepoContext): void {
    void appStore.switchContext(ctx.state.path);
  }

  function closeTab(ctx: RepoContext, e: MouseEvent): void {
    e.stopPropagation();
    void appStore.closeContext(ctx.state.path);
  }

  // Refresh worktree metadata whenever the window regains focus.
  function onFocus(): void {
    void appStore.refreshWorktrees();
  }
  onMount(() => {
    void appStore.refreshWorktrees();
    window.addEventListener("focus", onFocus);
  });
  onCleanup(() => window.removeEventListener("focus", onFocus));

  return (
    <div class="wt-strip">
      <For each={ordered()}>
        {(ctx) => {
          const meta = () => metaFor(ctx);
          const active = () => !!store.activeContextPath && samePath(store.activeContextPath, ctx.state.path);
          return (
            <div
              class="wt-tab"
              classList={{ "wt-tab--active": active(), "wt-tab--base": ctx.isBase }}
              title={`${projectOf(ctx)} — ${ctx.state.path}`}
              onClick={() => activate(ctx)}
            >
              <Show when={multiProject()}>
                <span
                  class="wt-project"
                  style={{
                    color: hueForSlug(projectOf(ctx)),
                    "border-color": `color-mix(in srgb, ${hueForSlug(projectOf(ctx))} 45%, transparent)`,
                    background: `color-mix(in srgb, ${hueForSlug(projectOf(ctx))} 12%, transparent)`,
                  }}
                  title={projectOf(ctx)}
                >
                  {projectInitials(projectOf(ctx))}
                </span>
              </Show>
              <svg class="wt-ico" viewBox="0 0 16 16" aria-hidden="true">
                <Show
                  when={ctx.isBase}
                  fallback={
                    <path
                      d="M4.5 2.5v7m0 0a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3m0-7a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3m7 0a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3m0 3v1.5a3 3 0 0 1-3 3H4.5"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.1"
                      stroke-linecap="round"
                    />
                  }
                >
                  <path
                    d="M2 4.2a1 1 0 0 1 1-1h3l1.2 1.3H13a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.1"
                  />
                </Show>
              </svg>
              <span class="wt-label">{label(ctx)}</span>
              <Show when={ctx.isBase}>
                <span class="wt-base-badge">base</span>
              </Show>
              <Show when={meta()?.dirty}>
                <span class="wt-dirty" title="uncommitted changes" />
              </Show>
              <Show when={(meta()?.ahead ?? 0) > 0}>
                <span class="wt-count wt-ahead" title={`${meta()!.ahead} ahead of base`}>↑{meta()!.ahead}</span>
              </Show>
              <Show when={(meta()?.behind ?? 0) > 0}>
                <span class="wt-count wt-behind" title={`${meta()!.behind} behind base`}>↓{meta()!.behind}</span>
              </Show>
              <Show when={!ctx.isBase}>
                <button class="wt-close" title="Close tab (keeps the worktree)" onClick={(e) => closeTab(ctx, e)}>
                  <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6">
                    <path d="M18 6L6 18M6 6l12 12" />
                  </svg>
                </button>
              </Show>
            </div>
          );
        }}
      </For>

      <div class="wt-new-wrap">
        <button
          ref={(el) => (newBtnRef = el)}
          class="wt-new"
          title="Worktrees & branches…"
          onClick={() => appStore.setWorktreeSwitcherOpen(!store.worktreeSwitcherOpen)}
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M12 5v14M5 12h14" />
          </svg>
        </button>
        <Show when={store.worktreeSwitcherOpen}>
          <WorktreeSwitcher anchor={() => newBtnRef} />
        </Show>
      </div>

      <Show when={store.worktreesLoading}>
        <span class="wt-loading" title="refreshing worktree status" />
      </Show>

      <style>{`
        .wt-strip {
          display: flex;
          align-items: stretch;
          height: 30px;
          padding: 4px 6px 0;
          gap: 3px;
          background: var(--bg-surface);
          border-bottom: 1px solid var(--border);
          flex-shrink: 0;
          overflow-x: auto;
          scrollbar-width: none;
        }
        .wt-strip::-webkit-scrollbar { display: none; }
        .wt-tab {
          display: flex; align-items: center; gap: 6px;
          padding: 0 8px 0 10px;
          max-width: 220px;
          font-size: 12px;
          color: var(--text-tertiary);
          border: 1px solid transparent;
          border-radius: var(--radius-sm) var(--radius-sm) 0 0;
          white-space: nowrap;
          cursor: pointer;
          flex-shrink: 0;
        }
        .wt-tab:hover { background: var(--bg-hover); color: var(--text-secondary); }
        .wt-tab--active {
          color: var(--text);
          background: var(--bg-base);
          border-color: var(--border);
          border-bottom-color: var(--bg-base);
          margin-bottom: -1px;
        }
        .wt-project {
          display: inline-flex; align-items: center; justify-content: center;
          height: 14px; padding: 0 3px;
          font-family: var(--font-mono); font-size: 8px; font-weight: 700;
          letter-spacing: 0.06em; line-height: 1;
          border: 1px solid; border-radius: 3px;
          flex-shrink: 0;
        }
        .wt-ico { width: 12px; height: 12px; flex-shrink: 0; color: var(--text-tertiary); }
        .wt-tab--active .wt-ico { color: var(--primary); }
        .wt-label {
          font-family: var(--font-mono); font-size: 11px;
          overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
          max-width: 150px;
        }
        .wt-base-badge {
          font-size: 8.5px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em;
          color: var(--text-tertiary);
          background: var(--bg-muted);
          border: 1px solid var(--border-variant);
          padding: 0 4px; border-radius: var(--radius-pill);
        }
        .wt-dirty { width: 6px; height: 6px; border-radius: 50%; background: var(--amber); flex-shrink: 0; }
        .wt-count {
          font-family: var(--font-mono); font-size: 10px; font-variant-numeric: tabular-nums;
          flex-shrink: 0;
        }
        .wt-ahead { color: var(--green); }
        .wt-behind { color: var(--orange); }
        .wt-close {
          display: flex; align-items: center; justify-content: center;
          width: 15px; height: 15px; border-radius: var(--radius-sm);
          color: var(--text-tertiary); opacity: 0; flex-shrink: 0;
        }
        .wt-tab:hover .wt-close, .wt-tab--active .wt-close { opacity: 0.7; }
        .wt-close:hover { opacity: 1; background: var(--bg-accent); color: var(--text); }

        .wt-new-wrap { position: relative; display: flex; align-self: center; }
        .wt-new {
          display: flex; align-items: center; justify-content: center;
          width: 22px; height: 22px; border-radius: var(--radius-sm);
          color: var(--text-tertiary);
        }
        .wt-new:hover { background: var(--bg-accent); color: var(--text-secondary); }
        .wt-loading {
          align-self: center; margin-left: 4px;
          width: 6px; height: 6px; border-radius: 50%;
          background: var(--sky); animation: dot-pulse 1.4s ease-in-out infinite;
        }
      `}</style>
    </div>
  );
}
