import { Match, Show, Switch, createMemo, onMount } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import type { LifecycleState } from "../../types";

/**
 * LifecycleBanner — the single place where the **lifecycle state** of a
 * thread (worktree / PR relationship) is rendered as persistent UI. Every
 * variant has its own icon, tone, copy, and actions. When the variant is
 * `working` (the default), the banner is hidden entirely.
 *
 * This is intentionally NOT a scroll-line system message. System messages in
 * the chat are events (a push happened, a review came in). State lives here.
 *
 *   - Composer reads `isLifecycleLocked` to decide whether to disable input.
 *   - Sidebar / TabBar read the lifecycle kind to decide which badge to show.
 *   - The poller writes to `store.lifecycleStates[threadId]`; no other code
 *     touches that record.
 *
 * ## Reactivity
 *
 * Earlier versions of this component used `<Show>` with a render-prop
 * callback that took a snapshot of `state()` into a local `const`. That
 * snapshot went stale when switching threads — the `<Show>` `when` stayed
 * truthy, so the callback never re-ran even though the thread id prop
 * changed. The fix is to use `<Switch>` + `<Match>` at the top level: each
 * `<Match>` is a top-level JSX node, so its `when` is tracked reactively and
 * the right branch is always rendered.
 */
export function LifecycleBanner(props: { threadId: string }) {
  const { store } = appStore;
  const state = createMemo<LifecycleState | undefined>(() => store.lifecycleStates[props.threadId]);

  // Narrowing accessors — each one re-reads the memo so SolidJS can track it.
  const kind = () => state()?.kind;
  const asOpen = () => (state()?.kind === "pr_open" ? state() as Extract<LifecycleState, { kind: "pr_open" }> : null);
  const asDiverged = () => (state()?.kind === "pr_open_diverged" ? state() as Extract<LifecycleState, { kind: "pr_open_diverged" }> : null);
  const asMerged = () => (state()?.kind === "pr_merged" ? state() as Extract<LifecycleState, { kind: "pr_merged" }> : null);
  const asClosed = () => (state()?.kind === "pr_closed" ? state() as Extract<LifecycleState, { kind: "pr_closed" }> : null);
  const asReverted = () => (state()?.kind === "pr_reverted" ? state() as Extract<LifecycleState, { kind: "pr_reverted" }> : null);
  const asMissing = () => (state()?.kind === "worktree_missing" ? state() as Extract<LifecycleState, { kind: "worktree_missing" }> : null);
  const asOrphaned = () => (state()?.kind === "worktree_orphaned" ? state() as Extract<LifecycleState, { kind: "worktree_orphaned" }> : null);

  onMount(() => {
    if (document.getElementById("lifecycle-banner-styles")) return;
    const s = document.createElement("style");
    s.id = "lifecycle-banner-styles";
    s.textContent = STYLES;
    document.head.appendChild(s);
  });

  function openUrl(url: string) {
    if (!url) return;
    // Open in the system browser via the Rust command. The Tauri webview
    // silently drops `window.open` for most URLs, so we can't rely on it.
    ipc.openExternalUrl(url).catch((e) => console.error("openExternalUrl:", e));
  }

  return (
    <Show when={kind() && kind() !== "working"}>
      <Switch>
        {/* ── PR OPEN — default in-progress state for a linked PR ── */}
        <Match when={asOpen()}>
          {(s) => (
            <div class="lb lb--open">
              <div class="lb-icon" aria-hidden="true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="18" cy="18" r="3" />
                  <circle cx="6" cy="6" r="3" />
                  <path d="M13 6h3a2 2 0 012 2v7" />
                  <line x1="6" y1="9" x2="6" y2="21" />
                </svg>
              </div>
              <div class="lb-body">
                <div class="lb-title">
                  PR #{s().pr.number} <span class="lb-pill lb-pill--open">open</span>
                  <Show when={s().ci && s().ci !== "none"}>
                    <span class={`lb-pill lb-pill--ci-${s().ci}`}>CI {s().ci}</span>
                  </Show>
                  <Show when={s().review !== "none"}>
                    <span class={`lb-pill lb-pill--review-${s().review}`}>
                      {s().review === "approved" ? "approved" : s().review === "changes_requested" ? "changes requested" : "reviewed"}
                    </span>
                  </Show>
                  <Show when={s().unread_comments > 0}>
                    <span class="lb-pill lb-pill--unread">{s().unread_comments} new</span>
                  </Show>
                </div>
              </div>
              <button class="lb-btn" onClick={() => openUrl(s().pr.url)}>View PR →</button>
            </div>
          )}
        </Match>

        {/* ── PR OPEN DIVERGED — remote has commits we don't ── */}
        <Match when={asDiverged()}>
          {(s) => (
            <div class="lb lb--warn">
              <div class="lb-icon" aria-hidden="true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M12 9v4M12 17h.01" /><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
                </svg>
              </div>
              <div class="lb-body">
                <div class="lb-title">PR #{s().pr.number} has diverged</div>
                <div class="lb-sub">
                  {s().ahead} commit{s().ahead === 1 ? "" : "s"} ahead, {s().behind} behind origin —
                  pull & rebase before pushing.
                </div>
              </div>
              <button class="lb-btn" onClick={() => openUrl(s().pr.url)}>View PR →</button>
            </div>
          )}
        </Match>

        {/* ── PR MERGED — terminal, thread locked ── */}
        <Match when={asMerged()}>
          {(s) => (
            <div class="lb lb--merged">
              <div class="lb-icon" aria-hidden="true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              </div>
              <div class="lb-body">
                <div class="lb-title">
                  PR #{s().pr.number} merged
                  <Show when={s().merge_commit}>
                    <span class="lb-mono">{s().merge_commit.slice(0, 8)}</span>
                  </Show>
                </div>
                <div class="lb-sub">Thread is read-only. Start a new thread to continue work.</div>
              </div>
              <button class="lb-btn" onClick={() => openUrl(s().pr.url)}>View PR →</button>
            </div>
          )}
        </Match>

        {/* ── PR CLOSED — terminal, thread locked ── */}
        <Match when={asClosed()}>
          {(s) => (
            <div class="lb lb--closed">
              <div class="lb-icon" aria-hidden="true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </div>
              <div class="lb-body">
                <div class="lb-title">PR #{s().pr.number} closed without merging</div>
                <div class="lb-sub">Thread is read-only. Reopen the PR on GitHub or start a new thread.</div>
              </div>
              <button class="lb-btn" onClick={() => openUrl(s().pr.url)}>View PR →</button>
            </div>
          )}
        </Match>

        {/* ── PR REVERTED — merge commit no longer reachable on base ── */}
        <Match when={asReverted()}>
          {(s) => (
            <div class="lb lb--warn">
              <div class="lb-icon" aria-hidden="true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M3 7v6h6" /><path d="M21 17a9 9 0 00-15-6.7L3 13" />
                </svg>
              </div>
              <div class="lb-body">
                <div class="lb-title">PR #{s().pr.number} appears reverted</div>
                <div class="lb-sub">
                  The merge commit is no longer reachable from the base branch.
                  This thread is editable — you can resume work here.
                </div>
              </div>
              <button class="lb-btn" onClick={() => openUrl(s().pr.url)}>View PR →</button>
            </div>
          )}
        </Match>

        {/* ── WORKTREE MISSING — directory gone ── */}
        <Match when={asMissing()}>
          {(s) => (
            <div class="lb lb--warn">
              <div class="lb-icon" aria-hidden="true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" /><line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
                </svg>
              </div>
              <div class="lb-body">
                <div class="lb-title">Worktree directory missing</div>
                <div class="lb-sub">
                  <span class="lb-mono">{s().path}</span> no longer exists on disk.
                </div>
              </div>
            </div>
          )}
        </Match>

        {/* ── WORKTREE ORPHANED — git worktree list doesn't know about it ── */}
        <Match when={asOrphaned()}>
          {(s) => (
            <div class="lb lb--warn">
              <div class="lb-icon" aria-hidden="true">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" /><line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
                </svg>
              </div>
              <div class="lb-body">
                <div class="lb-title">Worktree orphaned</div>
                <div class="lb-sub">
                  Branch <span class="lb-mono">{s().branch}</span> isn't tracked by git's
                  worktree list anymore.
                </div>
              </div>
            </div>
          )}
        </Match>
      </Switch>
    </Show>
  );
}

const STYLES = `
  .lb {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    margin: 0 var(--space-3) var(--space-2);
    border-radius: var(--radius-md);
    font-size: 12px;
    line-height: 1.35;
    border: 1px solid var(--border);
    background: var(--bg-muted);
    animation: lb-fade-in 0.18s ease both;
    flex-shrink: 0;
  }
  @keyframes lb-fade-in {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .lb-icon {
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-surface);
  }
  .lb-body { flex: 1; min-width: 0; }
  .lb-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    color: var(--text);
    flex-wrap: wrap;
  }
  .lb-sub {
    margin-top: 2px;
    color: var(--text-secondary);
    font-size: 11px;
  }
  .lb-mono {
    font-family: var(--font-mono);
    font-size: 10px;
    background: var(--bg-accent);
    color: var(--text-secondary);
    padding: 1px 6px;
    border-radius: 3px;
  }
  .lb-btn {
    flex-shrink: 0;
    padding: 4px 12px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    transition: all 0.12s;
  }
  .lb-btn:hover {
    background: var(--bg-hover);
    border-color: var(--border-strong);
  }
  .lb-pill {
    font-size: 9px;
    font-weight: 500;
    font-family: var(--font-mono);
    padding: 1px 6px;
    border-radius: var(--radius-pill);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .lb-pill--open {
    color: var(--primary);
    background: rgba(107, 124, 255, 0.12);
  }
  .lb-pill--ci-success {
    color: var(--green);
    background: rgba(76, 214, 148, 0.12);
  }
  .lb-pill--ci-failure {
    color: var(--red);
    background: rgba(242, 95, 103, 0.12);
  }
  .lb-pill--ci-pending {
    color: var(--amber);
    background: rgba(240, 184, 64, 0.12);
  }
  .lb-pill--ci-unknown,
  .lb-pill--ci-none {
    color: var(--text-tertiary);
    background: var(--bg-accent);
  }
  .lb-pill--review-approved {
    color: var(--green);
    background: rgba(76, 214, 148, 0.12);
  }
  .lb-pill--review-changes_requested {
    color: var(--amber);
    background: rgba(240, 184, 64, 0.12);
  }
  .lb-pill--review-commented {
    color: var(--text-secondary);
    background: var(--bg-accent);
  }
  .lb-pill--unread {
    color: var(--sky, #66b8e0);
    background: rgba(102, 184, 224, 0.14);
  }

  /* ── Variant accents ── */
  .lb--open {
    border-color: rgba(107, 124, 255, 0.35);
    background: linear-gradient(180deg, rgba(107, 124, 255, 0.06), var(--bg-muted));
  }
  .lb--open .lb-icon { color: var(--primary); }

  .lb--merged {
    border-color: rgba(76, 214, 148, 0.35);
    background: linear-gradient(180deg, rgba(76, 214, 148, 0.08), var(--bg-muted));
  }
  .lb--merged .lb-icon { color: var(--green); }

  .lb--closed {
    border-color: rgba(242, 95, 103, 0.35);
    background: linear-gradient(180deg, rgba(242, 95, 103, 0.08), var(--bg-muted));
  }
  .lb--closed .lb-icon { color: var(--red); }

  .lb--warn {
    border-color: rgba(240, 184, 64, 0.45);
    background: linear-gradient(180deg, rgba(240, 184, 64, 0.08), var(--bg-muted));
  }
  .lb--warn .lb-icon { color: var(--amber); }
`;
