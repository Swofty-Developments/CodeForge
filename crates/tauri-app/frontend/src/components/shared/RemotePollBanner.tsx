import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

const POLL_INTERVAL = 30_000; // 30 seconds

/**
 * Polls the remote for new commits on the active thread's branch.
 * For PR threads: checks the PR's branch.
 * For project threads: checks the current branch (usually main).
 * Shows a slide-down banner when behind, with a Pull button.
 */
export function RemotePollBanner() {
  const { store } = appStore;
  const [update, setUpdate] = createSignal<ipc.RemoteUpdate | null>(null);
  const [pulling, setPulling] = createSignal(false);
  const [dismissed, setDismissed] = createSignal(false);
  let timer: ReturnType<typeof setInterval> | undefined;

  /** Resolve the cwd for git operations — worktree path or project path. */
  function getCwd(): string | null {
    const tab = store.activeTab;
    if (!tab) return null;
    const wt = store.worktrees[tab];
    if (wt?.active) return wt.path;
    const proj = store.projects.find((p) => p.threads.some((t) => t.id === tab));
    if (proj && proj.path !== ".") return proj.path;
    return null;
  }

  /** Get the PR number for the active thread, if any. */
  function getPrNumber(): string | undefined {
    const tab = store.activeTab;
    if (!tab) return undefined;
    const proj = store.projects.find((p) => p.threads.some((t) => t.id === tab));
    if (!proj) return undefined;
    const prMap = store.projectPrMap[proj.id];
    const num = prMap?.[tab];
    return num ? String(num) : undefined;
  }

  async function poll() {
    const cwd = getCwd();
    if (!cwd) { setUpdate(null); return; }
    try {
      // Pass PR number so the backend resolves the real remote branch
      const result = await ipc.gitCheckRemote(cwd, undefined, getPrNumber());
      // Reset dismissed if the count changed (new commits arrived)
      if (result && dismissed()) {
        const prev = update();
        if (!prev || prev.behind !== result.behind) {
          setDismissed(false);
        }
      }
      setUpdate(result);
    } catch {
      // Silently ignore — we don't want poll failures to be noisy
    }
  }

  async function handlePull() {
    const cwd = getCwd();
    if (!cwd) return;
    setPulling(true);
    try {
      // Use git_pull_branch which explicitly pulls origin/<resolved branch>
      // Works correctly in worktrees that have no upstream tracking
      await ipc.gitPullBranch(cwd, undefined, getPrNumber());
      setUpdate(null);
      setDismissed(false);
    } catch (e) {
      const tab = store.activeTab;
      if (tab) {
        appStore.setStore("threadMessages", tab, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, content: `Pull failed: ${e}` },
        ]);
      }
    } finally {
      setPulling(false);
    }
  }

  onMount(() => {
    poll(); // Check immediately
    timer = setInterval(poll, POLL_INTERVAL);
  });

  onCleanup(() => {
    if (timer) clearInterval(timer);
  });

  // Re-poll when active tab changes
  let lastTab = store.activeTab;
  onMount(() => {
    const interval = setInterval(() => {
      if (store.activeTab !== lastTab) {
        lastTab = store.activeTab;
        setUpdate(null);
        setDismissed(false);
        poll();
      }
    }, 500);
    onCleanup(() => clearInterval(interval));
  });

  const visible = () => update() !== null && !dismissed();

  return (
    <>
      <Show when={visible()}>
        <div class="remote-poll-banner" classList={{ pulling: pulling() }}>
          <svg class="rpb-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="17 1 21 5 17 9" />
            <path d="M3 11V9a4 4 0 014-4h14" />
            <polyline points="7 23 3 19 7 15" />
            <path d="M21 13v2a4 4 0 01-4 4H3" />
          </svg>
          <span class="rpb-text">
            <strong>{update()!.behind}</strong> new commit{update()!.behind > 1 ? "s" : ""} on <code>{update()!.branch}</code>
          </span>
          <Show when={update()!.latest_message}>
            <span class="rpb-msg">{update()!.latest_message}</span>
          </Show>
          <button class="rpb-pull" onClick={handlePull} disabled={pulling()}>
            {pulling() ? "Pulling..." : "Pull"}
          </button>
          <button class="rpb-dismiss" onClick={() => setDismissed(true)} title="Dismiss">
            &times;
          </button>
        </div>
      </Show>
      <style>{`
        .remote-poll-banner {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 6px 12px;
          background: rgba(107, 124, 255, 0.08);
          border-bottom: 1px solid rgba(107, 124, 255, 0.15);
          font-size: 12px;
          color: var(--text-secondary);
          animation: rpb-slide-in 200ms cubic-bezier(0.16, 1, 0.3, 1) both;
          flex-shrink: 0;
        }
        @keyframes rpb-slide-in {
          from { opacity: 0; transform: translateY(-100%); }
          to { opacity: 1; transform: translateY(0); }
        }
        .rpb-icon {
          flex-shrink: 0;
          color: var(--primary);
        }
        .rpb-text {
          white-space: nowrap;
        }
        .rpb-text strong {
          color: var(--text);
        }
        .rpb-text code {
          font-family: var(--font-mono);
          font-size: 11px;
          background: var(--bg-accent);
          padding: 1px 5px;
          border-radius: 3px;
        }
        .rpb-msg {
          flex: 1;
          min-width: 0;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
          font-family: var(--font-mono);
          font-size: 11px;
          color: var(--text-tertiary);
          opacity: 0.8;
        }
        .rpb-pull {
          padding: 3px 12px;
          font-size: 11px;
          font-weight: 600;
          background: var(--primary);
          color: #fff;
          border-radius: 4px;
          flex-shrink: 0;
          transition: filter 0.1s;
        }
        .rpb-pull:hover:not(:disabled) {
          filter: brightness(1.15);
        }
        .rpb-pull:disabled {
          opacity: 0.6;
          cursor: default;
        }
        .rpb-dismiss {
          color: var(--text-tertiary);
          font-size: 16px;
          line-height: 1;
          padding: 2px;
          opacity: 0.5;
          transition: opacity 0.1s;
        }
        .rpb-dismiss:hover {
          opacity: 1;
        }
        .pulling .rpb-icon {
          animation: rpb-spin 1s linear infinite;
        }
        @keyframes rpb-spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
        @media (prefers-reduced-motion: reduce) {
          .remote-poll-banner { animation: none; }
          .pulling .rpb-icon { animation: none; }
        }
      `}</style>
    </>
  );
}
