import { Show, createMemo, createEffect, on, createSignal, onMount, onCleanup } from "solid-js";
import { appStore } from "../../stores/app-store";
import type { ThreadTokenUsage } from "../../types";

function formatTokenCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}k`;
  return String(n);
}

/**
 * Mini Canvas timeline — draws dots for each message in the thread.
 * User = white, Assistant = primary, System = tertiary, Tool = amber.
 */
function ActivityTimeline(props: { threadId: string }) {
  const { store } = appStore;
  let canvasRef: HTMLCanvasElement | undefined;

  function draw() {
    if (!canvasRef) return;
    const msgs = store.threadMessages[props.threadId];
    if (!msgs || msgs.length === 0) return;

    const ctx = canvasRef.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvasRef.clientWidth;
    const h = canvasRef.clientHeight;
    canvasRef.width = w * dpr;
    canvasRef.height = h * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    const dotR = 1.5;
    const gap = 4;
    const maxDots = Math.floor(w / gap);
    const visibleMsgs = msgs.slice(-maxDots);
    const startX = w - visibleMsgs.length * gap;

    for (let i = 0; i < visibleMsgs.length; i++) {
      const msg = visibleMsgs[i];
      const x = startX + i * gap + gap / 2;
      const y = h / 2;

      ctx.beginPath();
      ctx.arc(x, y, dotR, 0, Math.PI * 2);

      if (msg.role === "user") {
        ctx.fillStyle = "rgba(240, 238, 248, 0.7)";
      } else if (msg.role === "assistant") {
        ctx.fillStyle = "rgba(107, 124, 255, 0.8)";
      } else {
        ctx.fillStyle = "rgba(128, 126, 146, 0.5)";
      }
      ctx.fill();
    }
  }

  createEffect(() => {
    const _ = store.threadMessages[props.threadId]?.length;
    requestAnimationFrame(draw);
  });

  onMount(() => {
    const ro = new ResizeObserver(() => requestAnimationFrame(draw));
    if (canvasRef) ro.observe(canvasRef);
    onCleanup(() => ro.disconnect());
  });

  return <canvas ref={canvasRef} class="sb-timeline" />;
}

export function StatusBar() {
  const { store } = appStore;

  const sessionStatus = createMemo(() => {
    const tab = store.activeTab;
    if (!tab) return "idle";
    return store.runStates[tab] || "idle";
  });

  const statusLabel = createMemo(() => {
    const s = sessionStatus();
    if (s === "ready") return "Ready";
    if (s === "generating" || s === "starting") return "Generating";
    if (s === "error") return "Error";
    if (s === "interrupting") return "Stopping";
    return "Idle";
  });

  const statusColor = createMemo(() => {
    const s = sessionStatus();
    if (s === "ready") return "var(--green)";
    if (s === "generating" || s === "starting") return "var(--sky)";
    if (s === "error") return "var(--red)";
    return "var(--text-tertiary)";
  });

  const isGenerating = createMemo(() => {
    const s = sessionStatus();
    return s === "generating" || s === "starting";
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

  const shortModel = createMemo(() => {
    const m = modelName();
    if (!m) return null;
    return m
      .replace("claude-opus-4-6", "Opus 4.6")
      .replace("claude-sonnet-4-5", "Sonnet 4.5")
      .replace("claude-3-5-sonnet", "Sonnet 3.5")
      .replace("claude-3-opus", "Opus 3")
      .replace("claude-3-haiku", "Haiku 3")
      .replace("[1m]", " (1M)")
      .replace("(1m)", " (1M)");
  });

  // Track token delta for flash effect
  const [tokenDelta, setTokenDelta] = createSignal<string | null>(null);
  let prevTokens = 0;

  createEffect(on(
    () => tokenUsage()?.totalTokens,
    (total) => {
      if (!total) return;
      // Flash delta when generation completes
      if (prevTokens > 0 && total > prevTokens && !isGenerating()) {
        const delta = total - prevTokens;
        setTokenDelta(`+${formatTokenCount(delta)}`);
        setTimeout(() => setTokenDelta(null), 2000);
      }
      prevTokens = total;
    }
  ));

  // Activity line color based on state
  const lineColor = createMemo(() => {
    const s = sessionStatus();
    if (s === "generating" || s === "starting") return "var(--sky)";
    if (s === "error") return "var(--red)";
    if (s === "ready") return "var(--green)";
    return "transparent";
  });

  return (
    <Show when={store.activeTab}>
      <>
        {/* Live activity line — animated gradient during generation */}
        <div
          class="sb-activity-line"
          classList={{
            "sb-line-active": isGenerating(),
            "sb-line-flash": sessionStatus() === "ready" && prevTokens > 0,
          }}
          style={{ "--line-color": lineColor() }}
        />

        <div class="status-bar">
          <div class="sb-left">
            <span
              class="sb-status-dot"
              classList={{ "sb-dot-pulse": isGenerating() }}
              style={{ background: statusColor() }}
            />
            <span class="sb-label" style={{ color: statusColor() }}>{statusLabel()}</span>
            <Show when={providerName()}>
              <span class="sb-sep">/</span>
              <span class="sb-text">{providerName()}</span>
            </Show>
          </div>

          {/* Mini timeline — right of center */}
          <Show when={store.activeTab && !store.activeTab.startsWith("__")}>
            <ActivityTimeline threadId={store.activeTab!} />
          </Show>

          <div class="sb-right">
            <Show when={tokenUsage()}>
              {(usage) => (
                <span class="sb-tokens sb-mono" classList={{ "sb-tokens-ticking": isGenerating() }}>
                  {formatTokenCount(usage().totalTokens)}
                  <span class="sb-tokens-unit"> tok</span>
                </span>
              )}
            </Show>
            <Show when={tokenDelta()}>
              <span class="sb-delta">{tokenDelta()}</span>
            </Show>
            <Show when={shortModel()}>
              {(model) => (
                <>
                  <span class="sb-sep">&middot;</span>
                  <span class="sb-text sb-mono">{model()}</span>
                </>
              )}
            </Show>
          </div>
        </div>
        <style>{`
          /* ── Activity line — thin animated bar above status bar ── */
          .sb-activity-line {
            height: 1px;
            flex-shrink: 0;
            background: var(--line-color, transparent);
            transition: background 0.3s;
          }
          .sb-line-active {
            height: 2px;
            background: linear-gradient(
              90deg,
              transparent 0%,
              var(--sky) 30%,
              var(--primary) 50%,
              var(--sky) 70%,
              transparent 100%
            );
            background-size: 200% 100%;
            animation: sb-line-flow 1.5s linear infinite;
          }
          @keyframes sb-line-flow {
            from { background-position: 100% 0; }
            to { background-position: -100% 0; }
          }
          .sb-line-flash {
            height: 2px;
            animation: sb-line-done 0.6s ease-out both;
          }
          @keyframes sb-line-done {
            0% { opacity: 1; }
            50% { opacity: 1; background: var(--green); }
            100% { opacity: 0; height: 1px; }
          }

          /* ── Status bar ── */
          .status-bar {
            display: flex;
            align-items: center;
            justify-content: space-between;
            height: 24px;
            padding: 0 12px;
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
          .sb-label {
            font-size: 11px;
            font-weight: 600;
            white-space: nowrap;
            transition: color 0.2s;
          }
          .sb-text {
            font-size: 11px;
            color: var(--text-secondary);
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
            opacity: 0.4;
          }
          .sb-status-dot {
            width: 6px;
            height: 6px;
            border-radius: 50%;
            flex-shrink: 0;
            transition: background 0.2s;
          }
          .sb-dot-pulse {
            animation: sb-pulse 1.5s ease-in-out infinite;
          }
          @keyframes sb-pulse {
            0%, 100% { opacity: 1; transform: scale(1); }
            50% { opacity: 0.5; transform: scale(0.75); }
          }

          /* ── Token counter with ticking effect ── */
          .sb-tokens {
            font-size: 11px;
            color: var(--text-secondary);
            white-space: nowrap;
            font-variant-numeric: tabular-nums;
            transition: color 0.2s;
          }
          .sb-tokens-unit {
            color: var(--text-tertiary);
            font-size: 10px;
          }
          .sb-tokens-ticking {
            color: var(--sky);
          }

          /* ── Token delta flash ── */
          .sb-delta {
            font-size: 10px;
            font-family: var(--font-mono);
            font-weight: 600;
            color: var(--green);
            animation: sb-delta-in 0.3s ease-out both;
            white-space: nowrap;
          }
          @keyframes sb-delta-in {
            from { opacity: 0; transform: translateY(4px); }
            to { opacity: 1; transform: translateY(0); }
          }

          /* ── Mini activity timeline ── */
          .sb-timeline {
            flex: 1;
            min-width: 40px;
            max-width: 200px;
            height: 10px;
            opacity: 0.6;
          }

          /* ── prefers-reduced-motion ── */
          @media (prefers-reduced-motion: reduce) {
            .sb-line-active { animation: none; background: var(--sky); }
            .sb-line-flash { animation: none; }
            .sb-dot-pulse { animation: none; }
            .sb-delta { animation: none; }
          }
        `}</style>
      </>
    </Show>
  );
}
