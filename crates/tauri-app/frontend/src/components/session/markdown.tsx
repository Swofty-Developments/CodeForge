/* Markdown rendering for assistant text: marked + highlight.js, code blocks
 * with lang label + hover copy button, streaming last-line fade-up. Styles are
 * injected once (id guard) since this renders per message. */

import { createMemo } from "solid-js";
import { Marked, type Tokens } from "marked";
import hljs from "highlight.js";

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

const marked = new Marked({
  gfm: true,
  breaks: true,
  renderer: {
    code(token: Tokens.Code): string {
      const lang = (token.lang ?? "").split(/\s/)[0];
      let html: string;
      let label = lang;
      try {
        if (lang && hljs.getLanguage(lang)) {
          html = hljs.highlight(token.text, { language: lang }).value;
        } else {
          const auto = hljs.highlightAuto(token.text);
          html = auto.value;
          label = lang || auto.language || "";
        }
      } catch {
        html = escapeHtml(token.text);
      }
      return (
        `<div class="md-code-block">` +
        `<span class="md-code-lang">${escapeHtml(label)}</span>` +
        `<button class="md-copy-btn" type="button">Copy</button>` +
        `<pre><code class="hljs">${html}</code></pre>` +
        `</div>`
      );
    },
  },
});

function renderMarkdown(src: string): string {
  try {
    return marked.parse(src, { async: false });
  } catch {
    return `<p>${escapeHtml(src)}</p>`;
  }
}

/* Copy buttons live inside innerHTML, so handle them by delegation. */
function onRenderClick(e: MouseEvent): void {
  const btn = (e.target as HTMLElement).closest?.(".md-copy-btn");
  if (!(btn instanceof HTMLElement)) return;
  const code = btn.closest(".md-code-block")?.querySelector("code");
  void navigator.clipboard.writeText(code?.textContent ?? "").then(() => {
    btn.textContent = "Copied!";
    window.setTimeout(() => (btn.textContent = "Copy"), 1500);
  });
}

export function Markdown(props: { content: string; streaming?: boolean }) {
  injectMdStyles();
  const html = createMemo(() => renderMarkdown(props.content));
  return (
    <div
      class="md-render"
      classList={{ "md-render--streaming": !!props.streaming }}
      innerHTML={html()}
      onClick={onRenderClick}
    />
  );
}

function injectMdStyles(): void {
  if (document.getElementById("md-styles")) return;
  const style = document.createElement("style");
  style.id = "md-styles";
  style.textContent = `
.md-render { font-size: 14px; line-height: 1.6; color: var(--text); word-break: break-word; }
.md-render > *:first-child { margin-top: 0; }
.md-render > *:last-child { margin-bottom: 0; }
.md-render p { margin: 0.5em 0; }
.md-render h1 { font-size: 1.4em; margin: 0.8em 0 0.4em; letter-spacing: -0.3px; }
.md-render h2 { font-size: 1.25em; margin: 0.8em 0 0.4em; letter-spacing: -0.3px; }
.md-render h3, .md-render h4 { font-size: 1.1em; margin: 0.7em 0 0.35em; }
.md-render ul, .md-render ol { margin: 0.5em 0; padding-left: 1.4em; }
.md-render li { margin: 0.2em 0; }
.md-render a { color: var(--primary); text-decoration: none; }
.md-render a:hover { text-decoration: underline; }
.md-render hr { border: none; border-top: 1px solid var(--border); margin: 1em 0; }
.md-render code:not(.hljs) {
  background: rgba(255, 255, 255, 0.08);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 0.88em;
  color: var(--hljs-inline-code);
}
.md-render blockquote {
  margin: 0.75em 0; padding: 4px 16px;
  border-left: 3px solid var(--primary);
  color: var(--text-secondary);
  background: rgba(var(--primary-rgb), 0.04);
  border-radius: 0 4px 4px 0;
}
.md-render table { border-collapse: collapse; margin: 0.75em 0; font-size: 0.92em; }
.md-render th { background: rgba(255, 255, 255, 0.04); font-weight: 600; }
.md-render th, .md-render td { padding: 8px 12px; border: 1px solid rgba(255, 255, 255, 0.08); }

.md-code-block {
  position: relative;
  margin: 0.75em 0;
  border-radius: 8px;
  background: var(--hljs-code-bg);
  border: 1px solid var(--border);
  overflow: hidden;
}
.md-code-block pre { margin: 0; padding: 26px 16px 12px; overflow-x: auto; }
.md-code-block code.hljs {
  background: transparent;
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.5;
  color: var(--hljs-code-text);
}
.md-code-lang {
  position: absolute; top: 6px; left: 12px;
  font-size: 10px; color: var(--text-tertiary);
  text-transform: uppercase; letter-spacing: 0.5px;
  font-family: var(--font-mono);
}
.md-copy-btn {
  position: absolute; top: 4px; right: 8px;
  font-size: 11px; padding: 3px 10px; border-radius: 4px;
  background: rgba(255, 255, 255, 0.06);
  color: var(--text-tertiary);
  opacity: 0;
  transition: opacity 0.15s, background 0.15s, color 0.15s;
}
.md-code-block:hover .md-copy-btn { opacity: 1; }
.md-copy-btn:hover { background: rgba(255, 255, 255, 0.1); color: var(--text-secondary); }

/* Streaming: the newest line fades up as it appears */
.md-render--streaming > *:last-child { animation: streaming-line-in 0.15s var(--ease-out) both; }

/* hljs token colors route through the theme vars */
.hljs-keyword, .hljs-selector-tag, .hljs-type { color: var(--hljs-keyword); }
.hljs-string, .hljs-addition { color: var(--hljs-string); }
.hljs-number, .hljs-literal { color: var(--hljs-number); }
.hljs-comment, .hljs-deletion, .hljs-quote { color: var(--hljs-comment); font-style: italic; }
.hljs-built_in, .hljs-builtin-name { color: var(--hljs-builtin); }
.hljs-function .hljs-title, .hljs-title.function_ { color: var(--hljs-function); }
.hljs-attr, .hljs-attribute { color: var(--hljs-attr); }
.hljs-variable, .hljs-template-variable { color: var(--hljs-variable); }
.hljs-params { color: var(--hljs-params); }
.hljs-meta, .hljs-doctag { color: var(--hljs-meta); }
.hljs-regexp { color: var(--hljs-regexp); }
.hljs-tag { color: var(--hljs-tag); }
.hljs-selector-class, .hljs-selector-id { color: var(--hljs-selector); }
.hljs-symbol, .hljs-bullet { color: var(--hljs-symbol); }
.hljs-link { color: var(--hljs-link); }
.hljs-punctuation { color: var(--hljs-punctuation); }
.hljs-property { color: var(--hljs-property); }
.hljs-title.class_, .hljs-class .hljs-title { color: var(--hljs-class); }
`;
  document.head.appendChild(style);
}
