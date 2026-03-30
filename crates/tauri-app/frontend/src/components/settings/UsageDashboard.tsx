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
      <div class="usage-overlay-panel" onClick={(e) => e.stopPropagation()}>
        <div class="usage-dashboard">
          <div class="ud-header">
            <h3>Usage & Cost</h3>
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
            <div class="ud-status">Loading usage data...</div>
          </Show>
          <Show when={error()}>
            <div class="ud-status ud-error">{error()}</div>
          </Show>

          <Show when={!loading() && !error() && summary()}>
            {(() => {
              const s = summary()!;
              const totalTokens = s.total_input_tokens + s.total_output_tokens;
              return (
                <>
                  <div class="ud-hero">
                    <div class="ud-hero-cost">{formatCost(s.total_cost_usd)}</div>
                    <div class="ud-hero-label">total spend</div>
                  </div>

                  <div class="ud-token-grid">
                    <div class="ud-token-card">
                      <span class="ud-tv">{formatTokens(s.total_input_tokens)}</span>
                      <span class="ud-tl">Input</span>
                    </div>
                    <div class="ud-token-card">
                      <span class="ud-tv">{formatTokens(s.total_output_tokens)}</span>
                      <span class="ud-tl">Output</span>
                    </div>
                    <div class="ud-token-card">
                      <span class="ud-tv">{formatTokens(s.total_cache_read_tokens)}</span>
                      <span class="ud-tl">Cache Read</span>
                    </div>
                    <div class="ud-token-card">
                      <span class="ud-tv">{formatTokens(s.total_cache_write_tokens)}</span>
                      <span class="ud-tl">Cache Write</span>
                    </div>
                  </div>

                  <Show when={topThreads().length > 0}>
                    <div class="ud-section">
                      <h4>Cost by Thread</h4>
                      <div class="ud-bars">
                        <For each={topThreads()}>
                          {(thread) => {
                            const pct = () => Math.max((thread.cost_usd / maxThreadCost()) * 100, 1);
                            return (
                              <div class="ud-bar-row">
                                <div class="ud-bar-label" title={thread.thread_title}>
                                  {thread.thread_title.length > 28 ? thread.thread_title.slice(0, 28) + "..." : thread.thread_title}
                                </div>
                                <div class="ud-bar-track">
                                  <div class="ud-bar-fill" style={{ width: `${pct()}%` }} />
                                </div>
                                <div class="ud-bar-value">{formatCost(thread.cost_usd)}</div>
                              </div>
                            );
                          }}
                        </For>
                      </div>
                    </div>
                  </Show>

                  <Show when={s.model_costs.length > 0}>
                    <div class="ud-section">
                      <h4>By Model</h4>
                      <For each={s.model_costs}>
                        {(mc) => (
                          <div class="ud-model-row">
                            <span class="ud-model-name">{mc.model}</span>
                            <span class="ud-model-tokens">{formatTokens(mc.total_tokens)}</span>
                            <span class="ud-model-cost">{formatCost(mc.cost_usd)}</span>
                          </div>
                        )}
                      </For>
                    </div>
                  </Show>

                  <Show when={totalTokens === 0}>
                    <div class="ud-status">No usage data yet.</div>
                  </Show>
                </>
              );
            })()}
          </Show>
        </div>
      </div>

      <style>{`
        .usage-overlay-panel {
          background: var(--bg-card);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-lg);
          padding: 24px;
          max-width: 520px;
          width: 90%;
          max-height: 80vh;
          overflow-y: auto;
          box-shadow: 0 24px 80px rgba(0, 0, 0, 0.5), 0 0 1px rgba(255, 255, 255, 0.05);
          animation: overlay-panel-in 200ms cubic-bezier(0.16, 1, 0.3, 1) both;
        }
        .usage-dashboard { font-size: 13px; }
        .ud-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 16px;
        }
        .ud-header h3 {
          font-size: 16px;
          font-weight: 600;
          letter-spacing: -0.3px;
        }
        .ud-header-actions { display: flex; align-items: center; gap: 4px; }
        .ud-icon-btn {
          color: var(--text-tertiary);
          padding: 5px;
          border-radius: var(--radius-sm);
          display: flex;
          align-items: center;
          transition: background 0.12s, color 0.12s;
        }
        .ud-icon-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }

        .ud-status {
          text-align: center;
          padding: 32px 0;
          color: var(--text-tertiary);
          font-size: 13px;
        }
        .ud-error { color: var(--red); }

        .ud-hero { text-align: center; padding: 16px 0 14px; }
        .ud-hero-cost {
          font-size: 36px;
          font-weight: 700;
          letter-spacing: -1px;
          color: var(--green);
          line-height: 1;
          font-variant-numeric: tabular-nums;
        }
        .ud-hero-label {
          font-size: 10px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.08em;
          color: var(--text-tertiary);
          margin-top: 6px;
        }

        .ud-token-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 8px;
          margin-bottom: 20px;
        }
        .ud-token-card {
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: 8px;
          padding: 10px 12px;
          display: flex;
          flex-direction: column;
          gap: 2px;
        }
        .ud-tv {
          font-size: 16px;
          font-weight: 600;
          font-variant-numeric: tabular-nums;
        }
        .ud-tl {
          font-size: 10px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.06em;
          color: var(--text-tertiary);
        }

        .ud-section { margin-bottom: 18px; }
        .ud-section h4 {
          font-size: 10px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.06em;
          color: var(--text-tertiary);
          margin: 0 0 10px;
        }

        .ud-bars { display: flex; flex-direction: column; gap: 6px; }
        .ud-bar-row {
          display: grid;
          grid-template-columns: 130px 1fr 56px;
          align-items: center;
          gap: 8px;
        }
        .ud-bar-label {
          font-size: 12px;
          color: var(--text-secondary);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .ud-bar-track {
          height: 5px;
          background: var(--bg-accent);
          border-radius: 3px;
          overflow: hidden;
        }
        .ud-bar-fill {
          height: 100%;
          background: linear-gradient(90deg, var(--primary), var(--purple));
          border-radius: 3px;
          transition: width 0.3s ease;
        }
        .ud-bar-value {
          font-size: 12px;
          color: var(--green);
          text-align: right;
          font-variant-numeric: tabular-nums;
          font-weight: 500;
        }

        .ud-model-row {
          display: grid;
          grid-template-columns: 1fr auto auto;
          gap: 12px;
          padding: 6px 10px;
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: 6px;
          align-items: center;
          margin-bottom: 4px;
        }
        .ud-model-name {
          font-size: 12px;
          font-weight: 500;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .ud-model-tokens {
          font-size: 11px;
          color: var(--text-tertiary);
          font-variant-numeric: tabular-nums;
        }
        .ud-model-cost {
          font-size: 12px;
          color: var(--green);
          font-weight: 500;
          font-variant-numeric: tabular-nums;
        }
      `}</style>
    </div>
  );
}
