/* Worktree tab strip — one tab per open repo context (base first, then
 * worktrees). Each tab shows the branch, a dirty dot, and ahead/behind counts
 * from list_worktrees (refreshed on focus + after merges). The active tab is
 * Zed-styled and merges into the content surface. "+" opens the new-worktree
 * prompt; closing a worktree tab removes it (confirming when dirty). */

import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { appStore } from "../stores/app-store";
import type { RepoContext, Worktree } from "../types";
import { samePath } from "../stores/path";

export function WorktreeTabs() {
  const { store } = appStore;

  // Base first, then worktrees in open order (stable sort).
  const ordered = createMemo(() =>
    [...store.contexts].sort((a, b) => (a.isBase === b.isBase ? 0 : a.isBase ? -1 : 1)),
  );

  const metaFor = (ctx: RepoContext): Worktree | undefined =>
    store.worktrees.find((w) => samePath(String(w.path), ctx.state.path));

  const label = (ctx: RepoContext): string =>
    metaFor(ctx)?.branch ?? ctx.state.branch ?? ctx.state.name;

  function activate(ctx: RepoContext): void {
    void appStore.switchContext(ctx.state.path);
  }

  function closeTab(ctx: RepoContext, e: MouseEvent): void {
    e.stopPropagation();
    const meta = metaFor(ctx);
    const dirty = meta?.dirty ?? false;
    if (dirty) {
      const ok = window.confirm(
        `Worktree "${label(ctx)}" has uncommitted changes.\nRemove it anyway? The changes will be lost.`,
      );
      if (!ok) return;
    }
    void appStore.closeContext(ctx.state.path, dirty);
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

  // create_worktree branches off the ACTIVE context's HEAD, so label with it.
  const newFromBranch = createMemo(() => store.repo?.branch ?? store.repo?.name ?? "current branch");

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
              title={ctx.state.path}
              onClick={() => activate(ctx)}
            >
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
                <button class="wt-close" title="Remove worktree" onClick={(e) => closeTab(ctx, e)}>
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
          class="wt-new"
          title={`New worktree from ${newFromBranch()}`}
          onClick={() => appStore.setWorktreePromptOpen(!store.worktreePromptOpen)}
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M12 5v14M5 12h14" />
          </svg>
        </button>
        <Show when={store.worktreePromptOpen}>
          <NewWorktreePrompt baseRef={newFromBranch()} />
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

function NewWorktreePrompt(props: { baseRef: string }) {
  const [name, setName] = createSignal("");
  let inputRef: HTMLInputElement | undefined;
  onMount(() => queueMicrotask(() => inputRef?.focus()));

  function submit(): void {
    const n = name().trim();
    if (!n) return;
    appStore.setWorktreePromptOpen(false);
    void appStore.createWorktree(n);
  }

  return (
    <>
      <div class="wt-prompt-backdrop" onClick={() => appStore.setWorktreePromptOpen(false)} />
      <div class="wt-prompt" onClick={(e) => e.stopPropagation()}>
        <div class="wt-prompt-label">
          New worktree from <span class="wt-prompt-base">{props.baseRef || "current branch"}</span>
        </div>
        <input
          ref={inputRef}
          class="wt-prompt-input"
          placeholder="branch name…"
          value={name()}
          onInput={(e) => setName(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") { e.preventDefault(); submit(); }
            else if (e.key === "Escape") { e.preventDefault(); appStore.setWorktreePromptOpen(false); }
          }}
        />
        <div class="wt-prompt-actions">
          <button class="wt-prompt-cancel" onClick={() => appStore.setWorktreePromptOpen(false)}>Cancel</button>
          <button class="wt-prompt-create" disabled={!name().trim()} onClick={submit}>Create</button>
        </div>
      </div>

      <style>{`
        .wt-prompt-backdrop { position: fixed; inset: 0; z-index: 99; }
        .wt-prompt {
          position: absolute;
          top: calc(100% + 5px); right: 0;
          z-index: 100;
          width: 250px;
          padding: 10px;
          background: var(--bg-elevated);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-md);
          box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
          animation: dropdown-in 0.14s var(--ease-out);
          display: flex; flex-direction: column; gap: 8px;
        }
        .wt-prompt-label { font-size: 11px; color: var(--text-tertiary); }
        .wt-prompt-base { color: var(--primary); font-family: var(--font-mono); font-size: 10.5px; }
        .wt-prompt-input {
          width: 100%; font-family: var(--font-mono); font-size: 12px;
          background: var(--bg-muted); border: 1px solid var(--border);
          border-radius: var(--radius-sm); padding: 6px 8px; color: var(--text); outline: none;
        }
        .wt-prompt-input:focus { border-color: var(--border-glow); box-shadow: 0 0 0 2px var(--primary-glow); }
        .wt-prompt-actions { display: flex; justify-content: flex-end; gap: 6px; }
        .wt-prompt-cancel, .wt-prompt-create {
          font-size: 11.5px; font-weight: 600; padding: 4px 12px; border-radius: var(--radius-sm);
        }
        .wt-prompt-cancel { color: var(--text-secondary); border: 1px solid var(--border); }
        .wt-prompt-cancel:hover { background: var(--bg-accent); color: var(--text); }
        .wt-prompt-create { color: #16202e; background: var(--primary); }
        .wt-prompt-create:hover { filter: brightness(1.08); }
        .wt-prompt-create:disabled { opacity: 0.4; cursor: default; filter: none; }
      `}</style>
    </>
  );
}
