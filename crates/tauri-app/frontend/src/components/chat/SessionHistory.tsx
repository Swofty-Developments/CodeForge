import { createSignal, createResource, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { ThreadUsage } from "../../ipc";

interface Session {
  id: string;
  thread_id: string;
  provider: string;
  status: string;
  created_at: string;
  claude_session_id: string | null;
}

function relativeTime(iso: string): string {
  const now = Date.now();
  const then = new Date(iso).getTime();
  const diffSec = Math.floor((now - then) / 1000);
  if (diffSec < 60) return "just now";
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay < 30) return `${diffDay}d ago`;
  return new Date(iso).toLocaleDateString();
}

function providerLabel(provider: string): string {
  switch (provider.toLowerCase()) {
    case "claude": return "Claude";
    case "codex": return "Codex";
    default: return provider;
  }
}

function statusColor(status: string): string {
  switch (status.toLowerCase()) {
    case "running": return "var(--amber, #f0b840)";
    case "completed": case "done": return "var(--green, #4cd694)";
    case "error": case "failed": return "var(--red, #f25f67)";
    default: return "var(--text-tertiary)";
  }
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

function formatCost(usd: number): string {
  if (usd >= 1) return `$${usd.toFixed(2)}`;
  if (usd >= 0.01) return `$${usd.toFixed(3)}`;
  return `$${usd.toFixed(4)}`;
}

export function SessionHistory(props: { threadId: string }) {
  const [expanded, setExpanded] = createSignal(false);
  const [expandedSession, setExpandedSession] = createSignal<string | null>(null);

  const [sessions] = createResource(
    () => props.threadId,
    (tid) => invoke<Session[]>("get_sessions_by_thread", { threadId: tid })
  );

  const [usage] = createResource(
    () => (expandedSession() ? props.threadId : null),
    (tid) => tid ? invoke<ThreadUsage>("get_usage_for_thread", { threadId: tid }) : undefined
  );

  const toggleSession = (id: string) => {
    setExpandedSession((prev) => (prev === id ? null : id));
  };

  return (
    <>
      <div class="sh">
        <button class="sh-toggle" onClick={() => setExpanded(!expanded())}>
          <svg
            class="sh-chevron"
            classList={{ "sh-chevron--open": expanded() }}
            width="12" height="12" viewBox="0 0 24 24"
            fill="none" stroke="currentColor" stroke-width="2.5"
            stroke-linecap="round" stroke-linejoin="round"
          >
            <polyline points="9 18 15 12 9 6" />
          </svg>
          <span class="sh-title">Sessions</span>
          <Show when={sessions()}>
            <span class="sh-count">{sessions()!.length}</span>
          </Show>
        </button>

        <div class="sh-body" classList={{ "sh-body--open": expanded() }}>
          <div class="sh-body-inner">
            <Show when={sessions.loading}>
              <div class="sh-empty">Loading sessions...</div>
            </Show>
            <Show when={sessions.error}>
              <div class="sh-empty sh-error">Failed to load sessions</div>
            </Show>
            <Show when={sessions() && sessions()!.length === 0}>
              <div class="sh-empty">No sessions yet</div>
            </Show>
            <For each={sessions()}>
              {(session) => (
                <div class="sh-item">
                  <button
                    class="sh-item-header"
                    onClick={() => toggleSession(session.id)}
                  >
                    <span
                      class="sh-status-dot"
                      style={{ background: statusColor(session.status) }}
                    />
                    <span class="sh-provider" classList={{
                      "sh-provider--claude": session.provider.toLowerCase() === "claude",
                      "sh-provider--codex": session.provider.toLowerCase() === "codex",
                    }}>
                      {providerLabel(session.provider)}
                    </span>
                    <span class="sh-status-label">{session.status}</span>
                    <span class="sh-spacer" />
                    <Show when={session.claude_session_id}>
                      <span class="sh-session-id" title={session.claude_session_id!}>
                        {session.claude_session_id!.slice(0, 8)}
                      </span>
                    </Show>
                    <span class="sh-time">{relativeTime(session.created_at)}</span>
                    <svg
                      class="sh-item-chevron"
                      classList={{ "sh-item-chevron--open": expandedSession() === session.id }}
                      width="10" height="10" viewBox="0 0 24 24"
                      fill="none" stroke="currentColor" stroke-width="2.5"
                      stroke-linecap="round" stroke-linejoin="round"
                    >
                      <polyline points="9 18 15 12 9 6" />
                    </svg>
                  </button>

                  <div class="sh-detail" classList={{ "sh-detail--open": expandedSession() === session.id }}>
                    <div class="sh-detail-inner">
                      <Show when={usage.loading}>
                        <div class="sh-detail-row">Loading usage...</div>
                      </Show>
                      <Show when={usage.error}>
                        <div class="sh-detail-row sh-error">No usage data</div>
                      </Show>
                      <Show when={usage() && !usage.loading}>
                        {(() => {
                          const u = usage()!;
                          return (
                            <div class="sh-usage-grid">
                              <div class="sh-usage-item">
                                <span class="sh-usage-label">Input</span>
                                <span class="sh-usage-value">{formatTokens(u.input_tokens)}</span>
                              </div>
                              <div class="sh-usage-item">
                                <span class="sh-usage-label">Output</span>
                                <span class="sh-usage-value">{formatTokens(u.output_tokens)}</span>
                              </div>
                              <div class="sh-usage-item">
                                <span class="sh-usage-label">Cache read</span>
                                <span class="sh-usage-value">{formatTokens(u.cache_read_tokens)}</span>
                              </div>
                              <div class="sh-usage-item">
                                <span class="sh-usage-label">Cost</span>
                                <span class="sh-usage-value">{formatCost(u.cost_usd)}</span>
                              </div>
                            </div>
                          );
                        })()}
                      </Show>
                    </div>
                  </div>
                </div>
              )}
            </For>
          </div>
        </div>
      </div>

      <style>{`
        /* -- Session History -- */
        .sh {
          margin: 8px 0;
          border-radius: var(--radius-sm);
          background: rgba(255, 255, 255, 0.02);
          border: 1px solid var(--border);
          overflow: hidden;
        }

        .sh-toggle {
          display: flex;
          align-items: center;
          gap: 6px;
          width: 100%;
          padding: 7px 10px;
          cursor: pointer;
          transition: background 0.1s;
        }
        .sh-toggle:hover {
          background: rgba(255, 255, 255, 0.025);
        }

        .sh-chevron {
          flex-shrink: 0;
          color: var(--text-tertiary);
          opacity: 0.6;
          transition: transform 0.18s ease, opacity 0.15s;
        }
        .sh-toggle:hover .sh-chevron { opacity: 0.9; }
        .sh-chevron--open { transform: rotate(90deg); }

        .sh-title {
          font-size: 12px;
          font-weight: 600;
          color: var(--text);
        }

        .sh-count {
          font-size: 10px;
          font-weight: 600;
          font-family: var(--font-mono);
          color: var(--text-tertiary);
          background: var(--bg-surface);
          border-radius: 4px;
          padding: 1px 6px;
          min-width: 20px;
          text-align: center;
        }

        /* Expandable body */
        .sh-body {
          display: grid;
          grid-template-rows: 0fr;
          transition: grid-template-rows 0.2s ease;
        }
        .sh-body--open {
          grid-template-rows: 1fr;
        }
        .sh-body-inner {
          overflow: hidden;
        }

        .sh-empty {
          padding: 12px 10px;
          font-size: 11px;
          color: var(--text-tertiary);
          text-align: center;
        }
        .sh-error {
          color: var(--red, #f25f67);
        }

        /* Session item */
        .sh-item {
          border-top: 1px solid var(--border);
        }
        .sh-item-header {
          display: flex;
          align-items: center;
          gap: 8px;
          width: 100%;
          padding: 6px 10px;
          cursor: pointer;
          transition: background 0.1s;
        }
        .sh-item-header:hover {
          background: rgba(255, 255, 255, 0.025);
        }

        .sh-status-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          flex-shrink: 0;
        }

        .sh-provider {
          font-size: 11px;
          font-weight: 600;
          padding: 1px 6px;
          border-radius: 3px;
          white-space: nowrap;
        }
        .sh-provider--claude {
          color: var(--primary, #b47aff);
          background: rgba(180, 122, 255, 0.1);
        }
        .sh-provider--codex {
          color: var(--green, #4cd694);
          background: rgba(76, 214, 148, 0.1);
        }

        .sh-status-label {
          font-size: 10px;
          font-family: var(--font-mono);
          color: var(--text-tertiary);
        }

        .sh-spacer { flex: 1; min-width: 4px; }

        .sh-session-id {
          font-size: 10px;
          font-family: var(--font-mono);
          color: var(--text-tertiary);
          opacity: 0.6;
          cursor: default;
        }

        .sh-time {
          font-size: 10px;
          color: var(--text-tertiary);
          white-space: nowrap;
        }

        .sh-item-chevron {
          flex-shrink: 0;
          color: var(--text-tertiary);
          opacity: 0.4;
          transition: transform 0.18s ease, opacity 0.15s;
        }
        .sh-item-header:hover .sh-item-chevron { opacity: 0.7; }
        .sh-item-chevron--open { transform: rotate(90deg); }

        /* Expanded detail */
        .sh-detail {
          display: grid;
          grid-template-rows: 0fr;
          transition: grid-template-rows 0.2s ease;
        }
        .sh-detail--open {
          grid-template-rows: 1fr;
        }
        .sh-detail-inner {
          overflow: hidden;
        }
        .sh-detail-row {
          padding: 8px 10px;
          font-size: 11px;
          color: var(--text-secondary);
        }

        .sh-usage-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: var(--space-2, 8px);
          padding: 8px 10px 10px;
        }
        .sh-usage-item {
          display: flex;
          flex-direction: column;
          gap: 2px;
        }
        .sh-usage-label {
          font-size: 9px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.06em;
          color: var(--text-tertiary);
        }
        .sh-usage-value {
          font-size: 13px;
          font-weight: 600;
          font-family: var(--font-mono);
          color: var(--text);
        }
      `}</style>
    </>
  );
}
