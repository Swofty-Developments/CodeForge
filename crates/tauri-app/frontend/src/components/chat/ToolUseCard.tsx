import { createSignal, Show } from "solid-js";
import type { ContentBlock, ToolStatus } from "../../types";

/** SVG icon paths per tool category. */
function ToolIcon(props: { name: string; class?: string }) {
  const category = () => {
    switch (props.name) {
      case "Read": return "file-read";
      case "Write": return "file-write";
      case "Edit": return "file-edit";
      case "Glob": return "search-files";
      case "Grep": return "search-content";
      case "Bash": return "terminal";
      case "Agent": return "agent";
      case "WebSearch": case "WebFetch": return "globe";
      case "LSP": return "code";
      case "NotebookEdit": return "notebook";
      default: return "bolt";
    }
  };

  return (
    <svg class={props.class || "tool-icon-svg"} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      {(() => {
        switch (category()) {
          case "file-read":
            return <><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" /><polyline points="14 2 14 8 20 8" /><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" /></>;
          case "file-write":
            return <><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" /><polyline points="14 2 14 8 20 8" /><line x1="12" y1="18" x2="12" y2="12" /><polyline points="9 15 12 12 15 15" /></>;
          case "file-edit":
            return <><path d="M12 20h9" /><path d="M16.5 3.5a2.12 2.12 0 013 3L7 19l-4 1 1-4z" /></>;
          case "search-files":
            return <><circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" /><path d="M8 11h6" /><path d="M11 8v6" /></>;
          case "search-content":
            return <><circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" /></>;
          case "terminal":
            return <><polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" /></>;
          case "agent":
            return <><rect x="3" y="11" width="18" height="10" rx="2" /><circle cx="12" cy="5" r="3" /><path d="M7 11V8a5 5 0 0110 0v3" /></>;
          case "globe":
            return <><circle cx="12" cy="12" r="10" /><line x1="2" y1="12" x2="22" y2="12" /><path d="M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z" /></>;
          case "code":
            return <><polyline points="16 18 22 12 16 6" /><polyline points="8 6 2 12 8 18" /></>;
          case "notebook":
            return <><path d="M2 3h6a4 4 0 014 4v14a3 3 0 00-3-3H2z" /><path d="M22 3h-6a4 4 0 00-4 4v14a3 3 0 013-3h7z" /></>;
          default:
            return <><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" /></>;
        }
      })()}
    </svg>
  );
}

const TOOL_LABELS: Record<string, string> = {
  Read: "Read file",
  Write: "Write file",
  Edit: "Edit file",
  Glob: "Find files",
  Grep: "Search content",
  Bash: "Run command",
  Agent: "Sub-agent",
  WebSearch: "Web search",
  WebFetch: "Fetch URL",
  LSP: "Language server",
  NotebookEdit: "Edit notebook",
  TodoWrite: "Update tasks",
};

function parseToolInput(raw: string): Record<string, unknown> | null {
  try { return JSON.parse(raw); } catch { return null; }
}

function summarizeInput(toolName: string, input: Record<string, unknown> | null): string {
  if (!input) return "";
  switch (toolName) {
    case "Read": case "Write": case "Edit":
      return String(input.file_path || input.path || "");
    case "Bash":
      return String(input.command || input.description || "");
    case "Glob": case "Grep":
      return String(input.pattern || "");
    case "Agent":
      return String(input.description || input.prompt || "").slice(0, 80);
    case "WebSearch":
      return String(input.query || "");
    case "WebFetch":
      return String(input.url || "");
    default: {
      const first = Object.values(input).find((v) => typeof v === "string");
      return first ? String(first).slice(0, 80) : "";
    }
  }
}

function statusLabel(status: ToolStatus | undefined): string {
  switch (status) {
    case "generating": return "Preparing";
    case "running":    return "Running";
    case "completed":  return "Done";
    case "error":      return "Failed";
    default:           return "";
  }
}

function formatInput(toolName: string, parsed: Record<string, unknown> | null, raw: string): string {
  if (!parsed) return raw;
  switch (toolName) {
    case "Bash":
      return String(parsed.command || raw);
    case "Read":
      return String(parsed.file_path || raw);
    case "Write":
      return `${parsed.file_path || ""}\n───\n${String(parsed.content || "").slice(0, 500)}${String(parsed.content || "").length > 500 ? "\n…" : ""}`;
    case "Edit": {
      const fp = String(parsed.file_path || "");
      const old_ = String(parsed.old_string || "").slice(0, 200);
      const new_ = String(parsed.new_string || "").slice(0, 200);
      return `${fp}\n− ${old_}\n+ ${new_}`;
    }
    case "Grep": case "Glob":
      return `${parsed.pattern || ""}${parsed.path ? `  in ${parsed.path}` : ""}`;
    default:
      try { return JSON.stringify(parsed, null, 2); } catch { return raw; }
  }
}

function truncateOutput(output: string): string {
  if (output.length <= 1000) return output;
  return output.slice(0, 1000) + `\n\n… (${output.length - 1000} more chars)`;
}

/** Split tool output into stdout/stderr if the backend tagged stderr with [stderr]. */
function splitOutput(raw: string): { stdout: string; stderr: string } {
  const marker = "\n[stderr]\n";
  const idx = raw.indexOf(marker);
  if (idx === -1) return { stdout: raw, stderr: "" };
  return { stdout: raw.slice(0, idx), stderr: raw.slice(idx + marker.length) };
}

export function ToolUseCard(props: { block: ContentBlock }) {
  const [expanded, setExpanded] = createSignal(false);

  const toolName = () => props.block.tool_name || "";
  const label = () => TOOL_LABELS[toolName()] || toolName();
  const parsedInput = () => parseToolInput(props.block.tool_input || "");
  const summary = () => summarizeInput(toolName(), parsedInput());
  const isActive = () => props.block.tool_status === "generating" || props.block.tool_status === "running";
  const hasOutput = () => !!props.block.tool_output;
  const outputParts = () => splitOutput(props.block.tool_output || "");
  const status = () => props.block.tool_status;

  return (
    <div
      class="tc"
      classList={{
        "tc--active": isActive(),
        "tc--error": !!props.block.tool_error,
        "tc--done": status() === "completed",
      }}
    >
      <button class="tc-header" onClick={() => setExpanded(!expanded())}>
        <ToolIcon name={toolName()} class="tc-icon" />
        <span class="tc-label">{label()}</span>
        <Show when={summary() && !expanded()}>
          <span class="tc-summary">{summary()}</span>
        </Show>
        <span class="tc-spacer" />
        <span class="tc-status" classList={{
          "tc-status--active": isActive(),
          "tc-status--done": status() === "completed",
          "tc-status--error": status() === "error",
        }}>
          <Show when={isActive()}>
            <span class="tc-pulse" />
          </Show>
          {statusLabel(status())}
        </span>
        <svg
          class="tc-chevron"
          classList={{ "tc-chevron--open": expanded() }}
          width="12" height="12" viewBox="0 0 24 24"
          fill="none" stroke="currentColor" stroke-width="2.5"
          stroke-linecap="round" stroke-linejoin="round"
        >
          <polyline points="9 18 15 12 9 6" />
        </svg>
      </button>

      <div class="tc-body" classList={{ "tc-body--open": expanded() }}>
        <div class="tc-body-inner">
          <Show when={props.block.tool_input}>
            <div class="tc-section">
              <div class="tc-section-label">Input</div>
              <pre class="tc-code">{formatInput(toolName(), parsedInput(), props.block.tool_input || "")}</pre>
            </div>
          </Show>
          <Show when={hasOutput()}>
            <Show when={outputParts().stderr} fallback={
              <div class="tc-section">
                <div class="tc-section-label">{props.block.tool_error ? "Error" : "Output"}</div>
                <pre class="tc-code" classList={{ "tc-code--error": !!props.block.tool_error }}>
                  {truncateOutput(props.block.tool_output || "")}
                </pre>
              </div>
            }>
              <Show when={outputParts().stdout}>
                <div class="tc-section">
                  <div class="tc-section-label">stdout</div>
                  <pre class="tc-code">{truncateOutput(outputParts().stdout)}</pre>
                </div>
              </Show>
              <div class="tc-section">
                <div class="tc-section-label">stderr</div>
                <pre class="tc-code tc-code--error">{truncateOutput(outputParts().stderr)}</pre>
              </div>
            </Show>
          </Show>
        </div>
      </div>
    </div>
  );
}

if (!document.getElementById("tool-card-styles")) {
  const s = document.createElement("style");
  s.id = "tool-card-styles";
  s.textContent = `
    /* ── Tool Card ── */
    .tc {
      margin: 8px 0;
      border-radius: var(--radius-md);
      background: rgba(255, 255, 255, 0.02);
      border: 1px solid var(--border);
      overflow: hidden;
      transition: border-color 0.2s, box-shadow 0.2s;
    }
    .tc--active {
      border-color: rgba(240, 184, 64, 0.2);
      box-shadow: 0 0 12px -4px rgba(240, 184, 64, 0.08);
    }
    .tc--error {
      border-color: rgba(242, 95, 103, 0.25);
    }
    .tc--done {
      border-color: rgba(76, 214, 148, 0.12);
    }

    /* Header row */
    .tc-header {
      display: flex;
      align-items: center;
      gap: 8px;
      width: 100%;
      padding: 7px 10px;
      cursor: pointer;
      transition: background 0.1s;
    }
    .tc-header:hover {
      background: rgba(255, 255, 255, 0.025);
    }

    /* Icon */
    .tc-icon {
      width: 14px;
      height: 14px;
      flex-shrink: 0;
      color: var(--text-tertiary);
      transition: color 0.15s;
    }
    .tc--active .tc-icon { color: var(--amber); }
    .tc--done .tc-icon { color: var(--green); }
    .tc--error .tc-icon { color: var(--red); }

    /* Name */
    .tc-label {
      font-size: 12px;
      font-weight: 600;
      color: var(--text);
      white-space: nowrap;
      letter-spacing: -0.01em;
    }

    /* File path / command summary */
    .tc-summary {
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

    .tc-spacer { flex: 1; min-width: 4px; }

    /* Status badge */
    .tc-status {
      display: flex;
      align-items: center;
      gap: 5px;
      font-size: 10px;
      font-weight: 500;
      white-space: nowrap;
      color: var(--text-tertiary);
      font-family: var(--font-mono);
      letter-spacing: 0.02em;
    }
    .tc-status--active { color: var(--amber); }
    .tc-status--done { color: var(--green); }
    .tc-status--error { color: var(--red); }

    /* Pulsing dot for active state */
    .tc-pulse {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--amber);
      animation: tc-pulse-anim 1.5s ease-in-out infinite;
    }
    @keyframes tc-pulse-anim {
      0%, 100% { opacity: 1; transform: scale(1); }
      50% { opacity: 0.4; transform: scale(0.7); }
    }

    /* Chevron */
    .tc-chevron {
      flex-shrink: 0;
      color: var(--text-tertiary);
      opacity: 0.5;
      transition: transform 0.18s ease, opacity 0.15s;
    }
    .tc-header:hover .tc-chevron { opacity: 0.8; }
    .tc-chevron--open { transform: rotate(90deg); }

    /* Expandable body */
    .tc-body {
      display: grid;
      grid-template-rows: 0fr;
      transition: grid-template-rows 0.2s ease;
    }
    .tc-body--open {
      grid-template-rows: 1fr;
    }
    .tc-body-inner {
      overflow: hidden;
    }

    /* Sections inside expanded body */
    .tc-section {
      padding: 8px 10px;
      border-top: 1px solid var(--border);
    }
    .tc-section-label {
      font-size: 9px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--text-tertiary);
      margin-bottom: 4px;
    }
    .tc-code {
      font-family: var(--font-mono);
      font-size: 11.5px;
      line-height: 1.55;
      color: var(--text-secondary);
      background: var(--bg-base);
      border-radius: var(--radius-sm);
      padding: 8px 10px;
      overflow-x: auto;
      max-height: 280px;
      overflow-y: auto;
      white-space: pre-wrap;
      overflow-wrap: break-word;
      margin: 0;
    }
    .tc-code--error {
      color: var(--red);
    }
  `;
  document.head.appendChild(s);
}
