import { createSignal, Show } from "solid-js";
import { Markdown } from "./Markdown";

export function ThinkingBlock(props: { content: string; streaming?: boolean }) {
  const [expanded, setExpanded] = createSignal(false);

  const preview = () => {
    const text = props.content.replace(/\n+/g, " ").trim();
    if (text.length <= 100) return text;
    return text.slice(0, 100) + "…";
  };

  return (
    <div class="tb" classList={{ "tb--streaming": !!props.streaming }}>
      <button class="tb-header" onClick={() => setExpanded(!expanded())}>
        <svg
          class="tb-chevron"
          classList={{ "tb-chevron--open": expanded() }}
          width="12" height="12" viewBox="0 0 24 24"
          fill="none" stroke="currentColor" stroke-width="2.5"
          stroke-linecap="round" stroke-linejoin="round"
        >
          <polyline points="9 18 15 12 9 6" />
        </svg>
        <span class="tb-label">
          {props.streaming ? "Thinking" : "Thought"}
        </span>
        <Show when={props.streaming}>
          <span class="tb-dots"><span /><span /><span /></span>
        </Show>
        <Show when={!expanded() && preview()}>
          <span class="tb-preview">{preview()}</span>
        </Show>
      </button>

      <div class="tb-body" classList={{ "tb-body--open": expanded() }}>
        <div class="tb-body-inner">
          <div class="tb-content">
            <Markdown content={props.content} />
          </div>
        </div>
      </div>
    </div>
  );
}

if (!document.getElementById("thinking-styles")) {
  const s = document.createElement("style");
  s.id = "thinking-styles";
  s.textContent = `
    /* ── Thinking Block ── */
    .tb {
      margin: 8px 0;
      border-radius: var(--radius-md);
      background: rgba(180, 122, 255, 0.03);
      border: 1px solid rgba(180, 122, 255, 0.1);
      overflow: hidden;
      transition: border-color 0.2s;
    }
    .tb--streaming {
      border-color: rgba(180, 122, 255, 0.22);
    }

    /* Header */
    .tb-header {
      display: flex;
      align-items: center;
      gap: 6px;
      width: 100%;
      padding: 7px 10px;
      cursor: pointer;
      transition: background 0.1s;
    }
    .tb-header:hover {
      background: rgba(180, 122, 255, 0.04);
    }

    .tb-chevron {
      flex-shrink: 0;
      color: var(--purple);
      opacity: 0.7;
      transition: transform 0.18s ease, opacity 0.15s;
    }
    .tb-header:hover .tb-chevron { opacity: 1; }
    .tb-chevron--open { transform: rotate(90deg); }

    .tb-label {
      font-size: 12px;
      font-weight: 500;
      color: var(--purple);
      white-space: nowrap;
    }

    /* Streaming dots */
    .tb-dots {
      display: inline-flex;
      gap: 3px;
      margin-left: 1px;
    }
    .tb-dots span {
      width: 3.5px;
      height: 3.5px;
      border-radius: 50%;
      background: var(--purple);
      animation: tb-dot 1.4s infinite both;
    }
    .tb-dots span:nth-child(2) { animation-delay: 0.16s; }
    .tb-dots span:nth-child(3) { animation-delay: 0.32s; }
    @keyframes tb-dot {
      0%, 80%, 100% { opacity: 0.25; transform: scale(0.8); }
      40% { opacity: 1; transform: scale(1); }
    }

    /* Inline preview (shown collapsed, after label) */
    .tb-preview {
      flex: 1;
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 11.5px;
      color: var(--text-tertiary);
      font-style: italic;
      opacity: 0.7;
      margin-left: 4px;
    }

    /* Expandable body — uses grid trick for smooth height animation */
    .tb-body {
      display: grid;
      grid-template-rows: 0fr;
      transition: grid-template-rows 0.25s ease;
    }
    .tb-body--open {
      grid-template-rows: 1fr;
    }
    .tb-body-inner {
      overflow: hidden;
    }

    .tb-content {
      padding: 0 10px 10px;
    }
    .tb-content .md-render {
      font-size: 13px;
      color: var(--text-secondary);
    }
  `;
  document.head.appendChild(s);
}
