/* Status bar — 24px chrome: daemon dot, repo name, index age; right: sessions. */

import { Show, createMemo } from "solid-js";
import { appStore } from "../stores/app-store";
import type { DaemonState } from "../types";

/** Dot class + label + hover title for each named daemon state. `unknown` (an
 *  errored probe) is amber/indeterminate — never the definitive red "offline". */
function daemonView(d: DaemonState | null): { cls: string; label: string; title?: string } {
  if (!d) return { cls: "", label: "no daemon" };
  switch (d.kind) {
    case "running":
      return { cls: "status-dot--ready", label: d.port != null ? `daemon :${d.port}` : "daemon on" };
    case "offline":
      return { cls: "status-dot--error", label: "daemon off" };
    case "unknown":
      return { cls: "status-dot--waiting", label: "daemon status unknown", title: d.error };
  }
}

function indexAge(indexedAt: string | null): string {
  if (!indexedAt) return "not indexed";
  const ms = Date.now() - new Date(indexedAt).getTime();
  const mins = Math.floor(ms / 60_000);
  if (mins < 1) return "indexed just now";
  if (mins < 60) return `indexed ${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `indexed ${hours}h ago`;
  return `indexed ${Math.floor(hours / 24)}d ago`;
}

export function StatusBar() {
  const { store } = appStore;
  const generating = () => store.sessions.some((s) => s.runState === "generating");
  const daemon = createMemo(() => daemonView(store.daemon));

  return (
    <>
      <div
        class="sb-activity-line"
        classList={{ "sb-line-active": generating() }}
      />
      <div class="status-bar">
        <div class="sb-group">
          <span class={`status-dot ${daemon().cls}`} />
          <span class="sb-label sb-mono" title={daemon().title}>
            {daemon().label}
          </span>
        </div>
        <div class="sb-group">
          <Show when={store.repo} fallback={<span class="sb-label">no repo</span>}>
            {(repo) => (
              <>
                <span class="sb-label">{repo().name}</span>
                <span class="sb-sep">·</span>
                <span class="sb-label sb-mono sb-dim">{indexAge(repo().indexedAt)}</span>
              </>
            )}
          </Show>
        </div>
        <div class="sb-group">
          <span class="sb-label sb-mono sb-dim">
            {store.sessions.length === 0
              ? "no sessions"
              : `${store.sessions.length} session${store.sessions.length > 1 ? "s" : ""}`}
          </span>
        </div>
      </div>

      <style>{`
        .status-bar {
          display: flex; align-items: center; justify-content: space-between;
          height: 24px;
          padding: 0 12px;
          background: var(--bg-surface);
          border-top: 1px solid var(--border);
          flex-shrink: 0;
          user-select: none;
          gap: 8px;
        }
        .sb-group { display: flex; align-items: center; gap: 6px; min-width: 0; }
        .sb-label { font-size: 11px; font-weight: 500; white-space: nowrap; color: var(--text-secondary); }
        .sb-mono { font-family: var(--font-mono); font-weight: 500; }
        .sb-dim { color: var(--text-tertiary); }
        .sb-sep { color: var(--text-tertiary); opacity: 0.5; font-size: 11px; }

        /* Live activity line — 1px idle -> 2px flowing gradient while generating */
        .sb-activity-line {
          height: 1px;
          flex-shrink: 0;
          background: transparent;
          transition: background 0.3s;
        }
        .sb-line-active {
          height: 2px;
          background: linear-gradient(90deg, transparent 0%, var(--sky) 30%, var(--primary) 50%, var(--sky) 70%, transparent 100%);
          background-size: 200% 100%;
          animation: sb-line-flow 1.5s linear infinite;
        }
      `}</style>
    </>
  );
}
