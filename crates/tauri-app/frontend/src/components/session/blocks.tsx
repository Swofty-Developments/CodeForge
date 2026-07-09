/* Stream building blocks: tool cards, thinking blocks, typing indicator, and
 * the approval-request card. Status tints follow the semantic alpha formula. */

import { Show, createMemo, createSignal } from "solid-js";
import { appStore } from "../../stores/app-store";
import type { ContentBlock } from "../../types";

const SUMMARY_KEYS = ["file_path", "path", "command", "pattern", "query", "url", "description", "prompt"];

function toolSummary(inputJson: string | undefined): string {
  if (!inputJson) return "";
  try {
    const v = JSON.parse(inputJson) as Record<string, unknown>;
    for (const k of SUMMARY_KEYS) {
      const val = v[k];
      if (typeof val === "string" && val) return val;
    }
    const compact = JSON.stringify(v);
    return compact.length > 120 ? `${compact.slice(0, 120)}…` : compact;
  } catch {
    // Input streams in as partial JSON while generating.
    return inputJson.slice(0, 120);
  }
}

function prettyJson(raw: string | undefined): string {
  if (!raw) return "";
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

function truncateOutput(out: string): string {
  const max = 4000;
  return out.length > max ? `${out.slice(0, max)}\n… (${out.length - max} more chars)` : out;
}

function Chevron(props: { open: boolean }) {
  return (
    <svg
      class="sp-chevron"
      classList={{ "sp-chevron--open": props.open }}
      width="10" height="10" viewBox="0 0 24 24"
      fill="none" stroke="currentColor" stroke-width="2.5"
    >
      <path d="M9 18l6-6-6-6" />
    </svg>
  );
}

export function ToolCard(props: { block: ContentBlock }) {
  const [open, setOpen] = createSignal(false);
  const status = () => props.block.toolStatus ?? "completed";
  const active = () => status() === "generating" || status() === "running";
  const statusLabel = () =>
    status() === "generating" ? "generating" : status() === "running" ? "running" : status() === "error" ? "error" : "done";

  return (
    <div
      class="tc"
      classList={{ "tc--active": active(), "tc--done": status() === "completed", "tc--error": status() === "error" }}
    >
      <button class="tc-header" onClick={() => setOpen(!open())}>
        <Chevron open={open()} />
        <span class="tc-name" classList={{ "tc-name--unknown": !props.block.toolName }}>
          {props.block.toolName ?? "unknown tool"}
        </span>
        <span class="tc-summary">{toolSummary(props.block.toolInput)}</span>
        <Show when={active()}>
          <span class="tc-pulse" />
        </Show>
        <span class="tc-status" classList={{ "tc-status--error": status() === "error" }}>{statusLabel()}</span>
      </button>
      <div class="tc-body" classList={{ "tc-body--open": open() }}>
        <div class="tc-body-inner">
          <Show when={props.block.toolInput}>
            <div class="tc-section">
              <div class="tc-section-label">Input</div>
              <pre class="tc-code">{prettyJson(props.block.toolInput)}</pre>
            </div>
          </Show>
          <Show when={props.block.toolOutput}>
            <div class="tc-section">
              <div class="tc-section-label">Result</div>
              <pre class="tc-code" classList={{ "tc-code--error": !!props.block.toolError }}>
                {truncateOutput(props.block.toolOutput ?? "")}
              </pre>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}

export function ThinkingBlock(props: { block: ContentBlock; streaming: boolean }) {
  const [open, setOpen] = createSignal(false);
  const preview = () => props.block.content.replace(/\s+/g, " ").slice(0, 100);

  return (
    <div class="tb" classList={{ "tb--streaming": props.streaming }}>
      <button class="tc-header" onClick={() => setOpen(!open())}>
        <Chevron open={open()} />
        <span class="tb-label">{props.streaming ? "Thinking" : "Thought"}</span>
        <Show
          when={props.streaming}
          fallback={<span class="tb-preview">{preview()}</span>}
        >
          <span class="tb-dots"><span /><span /><span /></span>
        </Show>
      </button>
      <div class="tc-body" classList={{ "tc-body--open": open() }}>
        <div class="tc-body-inner">
          <div class="tb-full">{props.block.content}</div>
        </div>
      </div>
    </div>
  );
}

/** Three shimmer gradient lines shown while the agent is generating. */
export function TypingIndicator() {
  return (
    <div class="typing">
      <div class="typing-shimmer-line" />
      <div class="typing-shimmer-line" style={{ "animation-delay": "0.15s" }} />
      <div class="typing-shimmer-line short" style={{ "animation-delay": "0.3s" }} />
    </div>
  );
}

export function ApprovalCard(props: {
  sessionId: string;
  approval: { requestId: string; description: string };
}) {
  const [busy, setBusy] = createSignal(false);
  // Rust formats the description as "ToolName: {pretty input json}"; a missing
  // colon is a distinct "no tool name" state, not a fabricated one.
  const parsed = createMemo(() => {
    const d = props.approval.description;
    const i = d.indexOf(":");
    return i > 0 && i < 40
      ? { tool: d.slice(0, i) as string | null, input: d.slice(i + 1).trim() }
      : { tool: null as string | null, input: d };
  });

  async function respond(approve: boolean): Promise<void> {
    if (busy()) return;
    setBusy(true);
    // approveRequest clears pendingApproval itself; no second clear here.
    await appStore.approveRequest(props.sessionId, props.approval.requestId, approve);
  }

  return (
    <div class="approval-card">
      <div class="ac-header">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
          <line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
        <span class="ac-title">Permission required</span>
        <span class="ac-tool" classList={{ "ac-tool--unknown": !parsed().tool }}>
          {parsed().tool ?? "unknown tool"}
        </span>
      </div>
      <pre class="ac-input">{parsed().input}</pre>
      <div class="ac-actions">
        <button class="ac-deny" disabled={busy()} onClick={() => void respond(false)}>Deny</button>
        <button class="ac-approve" disabled={busy()} onClick={() => void respond(true)}>Approve</button>
      </div>
    </div>
  );
}
