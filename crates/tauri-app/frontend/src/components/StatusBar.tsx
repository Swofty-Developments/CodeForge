/* Status bar — the single, consolidated status surface (24px chrome). One
 * indicator on the left aggregates daemon + index freshness + indexing progress;
 * hovering it opens a detail popover with a Reindex action. Right side: session
 * count + terminal toggle. All status lives here — the title bar and sidebar no
 * longer carry their own dots. */

import { For, Show, createMemo } from "solid-js";
import { appStore } from "../stores/app-store";
import type { DaemonState, IndexStatus, RepoState } from "../types";

type Sev = "ok" | "busy" | "warn" | "bad" | "idle";

/** Aggregate the whole status into one dot severity + label. Order of concern:
 *  indexing in progress → daemon down → index stale/outdated → all good. */
function aggregate(
  repo: RepoState | null,
  daemon: DaemonState | null,
  status: IndexStatus | undefined,
  indexing: boolean,
): { sev: Sev; label: string } {
  if (!repo) return { sev: "idle", label: "no repo" };
  if (indexing) return { sev: "busy", label: "indexing…" };
  if (daemon?.kind === "offline") return { sev: "bad", label: "daemon off" };
  if (daemon?.kind === "unknown") return { sev: "warn", label: "daemon unknown" };
  if (status?.state === "outdated") return { sev: "warn", label: "index outdated" };
  if (status?.state === "stale") return { sev: "warn", label: "index stale" };
  if (status?.state === "never" || repo.indexedAt == null) return { sev: "warn", label: "not indexed" };
  return { sev: "ok", label: `${repo.featuresCount} features` };
}

function indexAge(indexedAt: string | null): string {
  if (!indexedAt) return "never";
  const mins = Math.floor((Date.now() - new Date(indexedAt).getTime()) / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

const FRESHNESS: Record<IndexStatus["state"], string> = {
  fresh: "up to date",
  stale: "files changed since indexing",
  outdated: "indexed by an older version",
  never: "not indexed yet",
};

export function StatusBar() {
  const { store } = appStore;
  const generating = () => store.sessions.some((s) => s.runState === "generating");
  const indexing = () => store.indexProgress != null && store.indexProgress.stage !== "error";
  const status = () => (store.activeContextPath ? store.indexStatusByPath[store.activeContextPath] : undefined);
  const agg = createMemo(() => aggregate(store.repo, store.daemon, status(), indexing()));
  const daemon = () => store.daemon;

  return (
    <>
      <div class="sb-activity-line" classList={{ "sb-line-active": generating() }} />
      <div class="status-bar">
        {/* ── The one consolidated status control (hover for detail) ── */}
        <div class="sb-status" tabindex="0">
          <span class={`status-dot sb-dot--${agg().sev}`} classList={{ "sb-dot--pulse": agg().sev === "busy" }} />
          <span class="sb-label">{agg().label}</span>
          <Show when={store.repo}>
            <span class="sb-caret">▴</span>
          </Show>

          <Show when={store.repo}>
            {(repo) => (
              <div class="sb-pop" role="tooltip">
                <div class="sb-pop-title">{repo().name}</div>
                <Show when={repo().branch}>
                  <div class="sb-pop-row"><span class="sb-pop-k">branch</span><span class="sb-pop-v sb-mono">{repo().branch}</span></div>
                </Show>
                <div class="sb-pop-row">
                  <span class="sb-pop-k">daemon</span>
                  <span class="sb-pop-v sb-mono">{daemonLabel(daemon())}</span>
                </div>
                <div class="sb-pop-row">
                  <span class="sb-pop-k">features</span>
                  <span class="sb-pop-v sb-mono">{repo().featuresCount}</span>
                </div>
                <div class="sb-pop-row">
                  <span class="sb-pop-k">indexed</span>
                  <span class="sb-pop-v sb-mono">{indexAge(repo().indexedAt)}</span>
                </div>
                <div class="sb-pop-row">
                  <span class="sb-pop-k">status</span>
                  <span class="sb-pop-v sb-mono">
                    {indexing() ? "indexing…" : FRESHNESS[status()?.state ?? (repo().indexedAt ? "fresh" : "never")]}
                  </span>
                </div>
                <Show when={status()?.state === "outdated"}>
                  <div class="sb-pop-row"><span class="sb-pop-k">version</span><span class="sb-pop-v sb-mono">v{status()?.indexVersion} → v{status()?.currentVersion}</span></div>
                </Show>
                <Show when={(status()?.changedFiles?.length ?? 0) > 0}>
                  <div class="sb-pop-changed">
                    <For each={status()!.changedFiles.slice(0, 4)}>{(f) => <div class="sb-pop-file sb-mono">{f}</div>}</For>
                    <Show when={status()!.changedFiles.length > 4}>
                      <div class="sb-pop-file sb-mono sb-dim">+{status()!.changedFiles.length - 4} more</div>
                    </Show>
                  </div>
                </Show>
                <button class="sb-pop-btn" disabled={indexing()} onClick={() => void appStore.reindex()}>
                  {indexing() ? "indexing…" : "Reindex"}
                </button>
              </div>
            )}
          </Show>
        </div>

        <div class="sb-group sb-right">
          <span class="sb-label sb-mono sb-dim">
            {store.sessions.length === 0 ? "no sessions" : `${store.sessions.length} session${store.sessions.length > 1 ? "s" : ""}`}
          </span>
          <button
            class="sb-term-btn"
            classList={{ "sb-term-btn--on": store.terminalPanelOpen }}
            title="Toggle terminal (⌘J)"
            disabled={!store.repo}
            onClick={() => appStore.toggleTerminalPanel()}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M5 7l4 4-4 4M11 15h6" />
            </svg>
            <Show when={store.terminals.length > 0}>
              <span class="sb-term-count">{store.terminals.length}</span>
            </Show>
          </button>
        </div>
      </div>

      <style>{`
        .status-bar {
          display: flex; align-items: center; justify-content: space-between;
          height: 24px; padding: 0 8px 0 10px;
          background: var(--bg-chrome);
          border-top: 1px solid var(--border);
          flex-shrink: 0; user-select: none; gap: 8px;
        }
        .sb-group { display: flex; align-items: center; gap: 6px; min-width: 0; }
        .sb-right { gap: 8px; }

        /* One status control + hover popover */
        .sb-status {
          position: relative;
          display: flex; align-items: center; gap: 6px;
          height: 18px; padding: 0 7px; border-radius: var(--radius-sm);
          cursor: default; outline: none;
        }
        .sb-status:hover, .sb-status:focus-visible { background: var(--bg-hover); }
        .sb-caret { font-size: 8px; color: var(--text-tertiary); margin-left: 1px; }
        .status-dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
        .sb-dot--ok   { background: var(--green); }
        .sb-dot--busy { background: var(--sky); }
        .sb-dot--warn { background: var(--amber); }
        .sb-dot--bad  { background: var(--red); }
        .sb-dot--idle { background: var(--text-tertiary); }
        .sb-dot--pulse { animation: dot-pulse 1.4s ease-in-out infinite; }

        .sb-pop {
          position: absolute; bottom: calc(100% + 8px); left: 0;
          min-width: 260px; padding: 10px 12px;
          background: var(--bg-elevated);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-lg);
          box-shadow: 0 12px 32px rgba(0,0,0,0.45);
          opacity: 0; transform: translateY(4px); pointer-events: none;
          transition: opacity 120ms var(--ease-out), transform 120ms var(--ease-out);
          z-index: 200;
        }
        .sb-status:hover .sb-pop, .sb-status:focus-within .sb-pop { opacity: 1; transform: translateY(0); pointer-events: auto; }
        .sb-pop-title { font-size: 12px; font-weight: 600; color: var(--text); margin-bottom: 8px; }
        .sb-pop-row { display: flex; justify-content: space-between; gap: 16px; padding: 2px 0; font-size: 11px; }
        .sb-pop-k { color: var(--text-tertiary); }
        .sb-pop-v { color: var(--text-secondary); }
        .sb-pop-changed {
          margin: 6px 0 2px; padding-top: 6px; border-top: 1px solid var(--border-variant);
          display: flex; flex-direction: column; gap: 2px;
        }
        .sb-pop-file { font-size: 10px; color: var(--text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
        .sb-pop-btn {
          margin-top: 10px; width: 100%; height: 24px;
          font-size: 11px; font-weight: 500; font-family: var(--font-mono);
          color: var(--primary); background: var(--bg-accent);
          border: 1px solid var(--border-glow); border-radius: var(--radius-sm);
        }
        .sb-pop-btn:hover:not(:disabled) { background: var(--primary-glow); }
        .sb-pop-btn:disabled { opacity: 0.6; }

        .sb-term-btn {
          display: flex; align-items: center; gap: 4px;
          height: 17px; padding: 0 5px; border-radius: var(--radius-sm);
          color: var(--text-tertiary);
        }
        .sb-term-btn:hover:not(:disabled) { background: var(--bg-accent); color: var(--text-secondary); }
        .sb-term-btn--on { color: var(--primary); }
        .sb-term-btn:disabled { opacity: 0.4; cursor: default; }
        .sb-term-count { font-family: var(--font-mono); font-size: 9.5px; font-weight: 600; font-variant-numeric: tabular-nums; line-height: 1; }

        .sb-label { font-size: 11px; font-weight: 500; white-space: nowrap; color: var(--text-secondary); }
        .sb-mono { font-family: var(--font-mono); font-weight: 500; }
        .sb-dim { color: var(--text-tertiary); }

        .sb-activity-line { height: 1px; flex-shrink: 0; background: transparent; }
        .sb-line-active {
          height: 2px;
          background: linear-gradient(90deg, transparent 0%, var(--sky) 30%, var(--primary) 50%, var(--sky) 70%, transparent 100%);
          background-size: 200% 100%;
          animation: sb-line-flow 1.5s linear infinite;
        }
        @media (prefers-reduced-motion: reduce) {
          .sb-dot--pulse, .sb-line-active { animation: none; }
        }
      `}</style>
    </>
  );
}

function daemonLabel(d: DaemonState | null): string {
  if (!d) return "—";
  switch (d.kind) {
    case "running": return d.port != null ? `on :${d.port}` : "on";
    case "offline": return "off";
    case "unknown": return "unknown";
  }
}
