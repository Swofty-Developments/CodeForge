import { createSignal, onMount, For, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import type { UsageSummary } from "../../ipc";

function formatCost(usd: number): string {
  if (usd >= 1) return `$${usd.toFixed(2)}`;
  if (usd >= 0.01) return `$${usd.toFixed(3)}`;
  return `$${usd.toFixed(4)}`;
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

export function UsageDashboard() {
  const { setStore } = appStore;
  const [summary, setSummary] = createSignal<UsageSummary | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  function close() { setStore("usageDashboardOpen", false); }

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const data = await ipc.getUsageSummary();
      setSummary(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  onMount(load);

  const topThreads = () => (summary()?.thread_costs ?? []).slice(0, 10);
  const maxThreadCost = () => {
    const threads = topThreads();
    if (threads.length === 0) return 1;
    return Math.max(...threads.map((t) => t.cost_usd), 0.0001);
  };

  return (
    <div class="overlay" onClick={close}>
      <div class="ud-panel" onClick={(e) => e.stopPropagation()}>
        <div class="ud-header">
          <h3>Usage</h3>
          <div class="ud-header-actions">
            <button class="ud-icon-btn" onClick={load} title="Refresh">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M23 4v6h-6M1 20v-6h6" />
                <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" />
              </svg>
            </button>
            <button class="ud-icon-btn" onClick={close}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
        </div>

        <Show when={loading()}>
          <div class="ud-empty">Loading...</div>
        </Show>
        <Show when={error()}>
          <div class="ud-empty ud-error">{error()}</div>
        </Show>

        <Show when={!loading() && !error() && summary()}>
          {(() => {
            const s = summary()!;
            const totalTokens = s.total_input_tokens + s.total_output_tokens;
            return (
              <>
                {/* Token summary — inline, compact */}
                <div class="ud-totals">
                  <span class="ud-total-cost">{formatCost(s.total_cost_usd)}</span>
                  <span class="ud-total-sep" />
                  <span class="ud-total-detail">{formatTokens(s.total_input_tokens)} in</span>
                  <span class="ud-total-detail">{formatTokens(s.total_output_tokens)} out</span>
                  <Show when={s.total_cache_read_tokens > 0}>
                    <span class="ud-total-detail ud-total-dim">{formatTokens(s.total_cache_read_tokens)} cached</span>
                  </Show>
                </div>

                {/* Thread breakdown */}
                <Show when={topThreads().length > 0}>
                  <table class="ud-table">
                    <thead>
                      <tr>
                        <th>Thread</th>
                        <th class="ud-th-right">Tokens</th>
                        <th class="ud-th-right">Cost</th>
                      </tr>
                    </thead>
                    <tbody>
                      <For each={topThreads()}>
                        {(thread) => {
                          const pct = () => Math.max((thread.cost_usd / maxThreadCost()) * 100, 2);
                          return (
                            <tr>
                              <td>
                                <div class="ud-thread-cell">
                                  <span class="ud-thread-name" title={thread.thread_title}>{thread.thread_title}</span>
                                  <div class="ud-bar">
                                    <div class="ud-bar-fill" style={{ width: `${pct()}%` }} />
                                  </div>
                                </div>
                              </td>
                              <td class="ud-td-mono">{formatTokens(thread.total_tokens ?? 0)}</td>
                              <td class="ud-td-cost">{formatCost(thread.cost_usd)}</td>
                            </tr>
                          );
                        }}
                      </For>
                    </tbody>
                  </table>
                </Show>

                {/* Model breakdown */}
                <Show when={s.model_costs.length > 0}>
                  <div class="ud-models">
                    <For each={s.model_costs}>
                      {(mc) => (
                        <span class="ud-model-tag">
                          {mc.model}
                          <span class="ud-model-cost">{formatCost(mc.cost_usd)}</span>
                        </span>
                      )}
                    </For>
                  </div>
                </Show>

                <Show when={totalTokens === 0}>
                  <div class="ud-empty">No usage data yet.</div>
                </Show>
              </>
            );
          })()}
        </Show>
      </div>

      <style>{`
        .ud-panel {
          background: var(--bg-card);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-lg);
          padding: var(--space-5);
          max-width: 480px;
          width: 90%;
          max-height: 80vh;
          overflow-y: auto;
          box-shadow: 0 24px 80px rgba(0, 0, 0, 0.5), 0 0 1px rgba(255, 255, 255, 0.05);
          animation: overlay-panel-in 200ms cubic-bezier(0.16, 1, 0.3, 1) both;
        }
        .ud-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: var(--space-4);
        }
        .ud-header h3 {
          font-size: 14px;
          font-weight: 600;
          letter-spacing: -0.2px;
        }
        .ud-header-actions { display: flex; align-items: center; gap: var(--space-1); }
        .ud-icon-btn {
          color: var(--text-tertiary);
          padding: var(--space-1);
          border-radius: var(--radius-sm);
          display: flex;
          align-items: center;
          transition: background 0.12s, color 0.12s;
        }
        .ud-icon-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }

        .ud-empty {
          text-align: center;
          padding: var(--space-8) 0;
          color: var(--text-tertiary);
          font-size: 13px;
        }
        .ud-error { color: var(--red); }

        /* ── Totals row ── */
        .ud-totals {
          display: flex;
          align-items: baseline;
          gap: var(--space-2);
          margin-bottom: var(--space-4);
          padding-bottom: var(--space-3);
          border-bottom: 1px solid var(--border);
        }
        .ud-total-cost {
          font-size: 18px;
          font-weight: 700;
          font-variant-numeric: tabular-nums;
          letter-spacing: -0.5px;
          color: var(--text);
        }
        .ud-total-sep {
          width: 1px;
          height: 14px;
          background: var(--border-strong);
          align-self: center;
        }
        .ud-total-detail {
          font-size: 11px;
          font-family: var(--font-mono);
          color: var(--text-secondary);
          font-variant-numeric: tabular-nums;
        }
        .ud-total-dim { color: var(--text-tertiary); }

        /* ── Thread table ── */
        .ud-table {
          width: 100%;
          border-collapse: collapse;
          margin-bottom: var(--space-4);
        }
        .ud-table thead th {
          font-size: 10px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.06em;
          color: var(--text-tertiary);
          padding: 0 0 var(--space-2);
          text-align: left;
          border-bottom: 1px solid var(--border);
        }
        .ud-th-right { text-align: right !important; }
        .ud-table tbody tr {
          transition: background 0.08s;
        }
        .ud-table tbody tr:hover {
          background: var(--bg-hover);
        }
        .ud-table td {
          padding: var(--space-2) 0;
          font-size: 12px;
          border-bottom: 1px solid var(--border);
          vertical-align: middle;
        }
        .ud-thread-cell {
          display: flex;
          flex-direction: column;
          gap: 3px;
          padding-right: var(--space-3);
        }
        .ud-thread-name {
          color: var(--text-secondary);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
          max-width: 260px;
        }
        .ud-bar {
          height: 2px;
          background: var(--bg-accent);
          border-radius: 1px;
          overflow: hidden;
        }
        .ud-bar-fill {
          height: 100%;
          background: var(--primary);
          border-radius: 1px;
          transition: width 0.3s ease;
        }
        .ud-td-mono {
          font-family: var(--font-mono);
          font-size: 11px;
          color: var(--text-tertiary);
          text-align: right;
          font-variant-numeric: tabular-nums;
        }
        .ud-td-cost {
          font-family: var(--font-mono);
          font-size: 11px;
          color: var(--text-secondary);
          text-align: right;
          font-variant-numeric: tabular-nums;
          font-weight: 500;
        }

        /* ── Model tags ── */
        .ud-models {
          display: flex;
          flex-wrap: wrap;
          gap: var(--space-2);
        }
        .ud-model-tag {
          display: flex;
          align-items: center;
          gap: var(--space-2);
          font-size: 11px;
          color: var(--text-secondary);
          padding: var(--space-1) var(--space-2);
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: var(--radius-sm);
        }
        .ud-model-cost {
          font-family: var(--font-mono);
          font-size: 10px;
          color: var(--text-tertiary);
          font-variant-numeric: tabular-nums;
        }
      `}</style>
    </div>
  );
}
