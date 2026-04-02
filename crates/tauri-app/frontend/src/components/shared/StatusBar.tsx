import { Show, createMemo } from "solid-js";
import { appStore } from "../../stores/app-store";
import type { ThreadTokenUsage } from "../../types";

function formatTokenCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}k`;
  return String(n);
}

export function StatusBar() {
  const { store } = appStore;

  const sessionStatus = createMemo(() => {
    const tab = store.activeTab;
    if (!tab) return "idle";
    return store.sessionStatuses[tab] || "idle";
  });

  const statusLabel = createMemo(() => {
    const s = sessionStatus();
    if (s === "ready") return "Ready";
    if (s === "generating" || s === "starting") return "Generating...";
    if (s === "error") return "Error";
    if (s === "interrupting") return "Stopping...";
    return "Idle";
  });

  const statusColor = createMemo(() => {
    const s = sessionStatus();
    if (s === "ready") return "var(--green)";
    if (s === "generating" || s === "starting") return "var(--sky)";
    if (s === "error") return "var(--red)";
    return "var(--text-tertiary)";
  });

  const tokenUsage = createMemo((): ThreadTokenUsage | undefined => {
    const tab = store.activeTab;
    return tab ? store.threadTokenUsage[tab] : undefined;
  });

  const modelName = createMemo(() => {
    const usage = tokenUsage();
    if (usage?.model) return usage.model;
    if (store.activeModel) return store.activeModel;
    return null;
  });

  const providerName = createMemo(() => {
    const p = store.selectedProvider;
    if (p === "claude_code") return "Claude Code";
    if (p === "anthropic") return "Anthropic";
    if (p === "openai") return "OpenAI";
    return p;
  });

  // Shorten model identifier for display
  const shortModel = createMemo(() => {
    const m = modelName();
    if (!m) return null;
    // Remove common prefixes for compactness
    return m
      .replace("claude-opus-4-6", "Opus 4.6")
      .replace("claude-sonnet-4-5", "Sonnet 4.5")
      .replace("claude-3-5-sonnet", "Sonnet 3.5")
      .replace("claude-3-opus", "Opus 3")
      .replace("claude-3-haiku", "Haiku 3")
      .replace("[1m]", " (1M)")
      .replace("(1m)", " (1M)");
  });

  return (
    <Show when={store.activeTab}>
      <div class="status-bar">
        <div class="sb-left">
          <span class="sb-status-dot" style={{ background: statusColor() }} />
          <span class="sb-text">{statusLabel()}</span>
          <Show when={providerName()}>
            <span class="sb-sep">|</span>
            <span class="sb-text">{providerName()}</span>
          </Show>
        </div>
        <div class="sb-right">
          <Show when={tokenUsage()}>
            {(usage) => (
              <span class="sb-text sb-mono">{formatTokenCount(usage().totalTokens)} tokens</span>
            )}
          </Show>
          <Show when={shortModel()}>
            {(model) => (
              <>
                <span class="sb-sep">|</span>
                <span class="sb-text sb-mono">{model()}</span>
              </>
            )}
          </Show>
        </div>
      </div>
    </Show>
  );
}

if (!document.getElementById("status-bar-styles")) {
  const s = document.createElement("style");
  s.id = "status-bar-styles";
  s.textContent = `
    .status-bar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      height: 20px;
      padding: 0 10px;
      background: var(--bg-surface);
      border-top: 1px solid var(--border);
      flex-shrink: 0;
      user-select: none;
      gap: 8px;
    }
    .sb-left, .sb-right {
      display: flex;
      align-items: center;
      gap: 6px;
      min-width: 0;
    }
    .sb-text {
      font-size: 10px;
      color: var(--text-tertiary);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .sb-mono {
      font-family: var(--font-mono);
    }
    .sb-sep {
      font-size: 10px;
      color: var(--text-tertiary);
      opacity: 0.35;
    }
    .sb-status-dot {
      width: 5px;
      height: 5px;
      border-radius: 50%;
      flex-shrink: 0;
    }
  `;
  document.head.appendChild(s);
}
