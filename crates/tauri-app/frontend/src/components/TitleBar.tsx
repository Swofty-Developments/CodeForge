/* Title bar — Zed-style IDE chrome. Draggable, macOS traffic-light inset on the
 * left, then the project + branch + feature-count pills. Right side carries the
 * reindex control. Pills are the primary identity affordance top-left. */

import { Show, createMemo, createSignal } from "solid-js";
import { appStore } from "../stores/app-store";
import { samePath } from "../stores/path";

export function TitleBar() {
  const { store } = appStore;
  const repo = () => store.repo;

  const [branchMenu, setBranchMenu] = createSignal(false);

  const activeCtx = () =>
    store.activeContextPath
      ? store.contexts.find((c) => samePath(c.state.path, store.activeContextPath!)) ?? null
      : null;
  const activeIsBase = () => activeCtx()?.isBase ?? true;
  const baseBranch = createMemo(() => {
    const base = store.contexts.find((c) => c.isBase) ?? store.contexts[0];
    return base ? base.state.branch ?? base.state.name : "";
  });

  function newWorktree(): void {
    setBranchMenu(false);
    appStore.setWorktreePromptOpen(true);
  }
  function mergeActive(): void {
    setBranchMenu(false);
    const p = store.activeContextPath;
    if (p) void appStore.mergeWorktree(p);
  }

  return (
    <div class="titlebar" data-tauri-drag-region>
      <div class="tb-traffic" />
      <Show
        when={repo()}
        fallback={<span class="tb-appname">CodeForge</span>}
      >
        {(r) => (
          <div class="tb-pills">
            <button
              class="tb-pill tb-pill--project"
              title={r().path}
              onClick={() => appStore.setPaletteOpen(true)}
            >
              <svg class="tb-ico" viewBox="0 0 16 16" aria-hidden="true">
                <path
                  d="M1.5 3.5a1 1 0 0 1 1-1h3l1.2 1.4h6.3a1 1 0 0 1 1 1v6.6a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.1"
                />
              </svg>
              <span class="tb-pill-label">{r().name}</span>
            </button>

            <div class="tb-branch-wrap">
              <button
                class="tb-pill tb-pill--branch"
                classList={{ "tb-pill--open": branchMenu() }}
                title="branch actions"
                onClick={() => setBranchMenu((o) => !o)}
              >
                <svg class="tb-ico" viewBox="0 0 16 16" aria-hidden="true">
                  <path
                    d="M4.5 2.5v7m0 0a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3m0-7a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3m7 0a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3m0 3v1.5a3 3 0 0 1-3 3H4.5"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.1"
                    stroke-linecap="round"
                  />
                </svg>
                <span class="tb-pill-label tb-mono">{r().branch ?? "detached"}</span>
                <svg class="tb-caret" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M4 6l4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
                </svg>
              </button>
              <Show when={branchMenu()}>
                <div class="tb-menu-backdrop" onClick={() => setBranchMenu(false)} />
                <div class="tb-branch-menu">
                  <button class="tb-menu-item" onClick={newWorktree}>
                    New worktree from <span class="tb-menu-branch">{r().branch ?? "HEAD"}</span>…
                  </button>
                  <Show when={!activeIsBase()}>
                    <button class="tb-menu-item" onClick={mergeActive}>
                      Merge <span class="tb-menu-branch">{r().branch ?? "HEAD"}</span> into{" "}
                      <span class="tb-menu-branch">{baseBranch()}</span>
                    </button>
                  </Show>
                </div>
              </Show>
            </div>

            <div class="tb-pill tb-pill--count" title="features in this repo">
              <span class="tb-pill-label tb-mono">{r().featuresCount}</span>
              <span class="tb-pill-sub">
                {r().featuresCount === 1 ? "feature" : "features"}
              </span>
            </div>
          </div>
        )}
      </Show>

      <div class="tb-right" />

      <style>{`
        .titlebar {
          height: var(--titlebar-height);
          flex-shrink: 0;
          display: flex;
          align-items: center;
          gap: var(--space-2);
          padding: 0 var(--space-3) 0 0;
          background: var(--bg-chrome);
          border-bottom: 1px solid var(--border);
          -webkit-app-region: drag;
        }
        /* macOS traffic-light inset (window controls overlay this space). */
        .tb-traffic { width: 72px; flex-shrink: 0; height: 100%; }
        .tb-appname {
          font-size: 12px; font-weight: 600; letter-spacing: -0.2px;
          color: var(--text-secondary);
        }
        .tb-pills { display: flex; align-items: center; gap: var(--space-2); min-width: 0; }
        .tb-pill {
          display: inline-flex; align-items: center; gap: 6px;
          height: 22px; padding: 0 9px;
          border-radius: var(--radius-sm);
          background: var(--element-bg, var(--bg-muted));
          border: 1px solid var(--border-variant);
          color: var(--text-secondary);
          font-size: 12px; line-height: 1;
          -webkit-app-region: no-drag;
          max-width: 260px;
        }
        button.tb-pill { cursor: default; }
        button.tb-pill:hover { background: var(--bg-hover); color: var(--text); }
        .tb-pill--project .tb-pill-label { color: var(--text); font-weight: 500; }
        .tb-pill-label {
          overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
        }
        .tb-mono { font-family: var(--font-mono); font-size: 11px; }
        .tb-pill--branch { color: var(--text-muted); cursor: pointer; }
        .tb-pill--branch .tb-ico { color: var(--text-tertiary); }
        .tb-pill--open, .tb-pill--branch:hover { background: var(--bg-hover); color: var(--text); }
        .tb-caret { width: 11px; height: 11px; flex-shrink: 0; color: var(--text-tertiary); opacity: 0.7; }
        .tb-branch-wrap { position: relative; display: inline-flex; -webkit-app-region: no-drag; }
        .tb-menu-backdrop { position: fixed; inset: 0; z-index: 99; }
        .tb-branch-menu {
          position: absolute;
          top: calc(100% + 5px); left: 0;
          z-index: 100;
          min-width: 220px;
          padding: 4px;
          background: var(--bg-elevated);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-md);
          box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
          animation: dropdown-in 0.14s var(--ease-out);
        }
        .tb-menu-item {
          display: block; width: 100%; text-align: left;
          padding: 7px 9px; border-radius: var(--radius-sm);
          font-size: 12px; color: var(--text-secondary); white-space: nowrap;
        }
        .tb-menu-item:hover { background: var(--bg-accent); color: var(--text); }
        .tb-menu-branch { font-family: var(--font-mono); font-size: 11px; color: var(--primary); }
        .tb-pill--count { gap: 4px; }
        .tb-pill--count .tb-pill-label { color: var(--primary); font-weight: 600; }
        .tb-pill-sub { color: var(--text-tertiary); font-size: 11px; }
        .tb-ico { width: 13px; height: 13px; flex-shrink: 0; }

        .tb-right { margin-left: auto; display: flex; align-items: center; -webkit-app-region: no-drag; }
      `}</style>
    </div>
  );
}
