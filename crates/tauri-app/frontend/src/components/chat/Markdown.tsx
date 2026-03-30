import { createMemo } from "solid-js";
import { Marked, Renderer } from "marked";
import hljs from "highlight.js";

// Custom renderer with syntax highlighting and copy buttons
const renderer = new Renderer();

renderer.code = function ({ text, lang }: { text: string; lang?: string; escaped?: boolean }): string {
  const language = lang || "";
  let highlighted: string;
  if (language && hljs.getLanguage(language)) {
    try {
      highlighted = hljs.highlight(text, { language }).value;
    } catch {
      highlighted = escapeHtml(text);
    }
  } else {
    try {
      highlighted = hljs.highlightAuto(text).value;
    } catch {
      highlighted = escapeHtml(text);
    }
  }
  const langLabel = language ? `<span class="md-code-lang">${language}</span>` : "";
  return `<div class="md-code-block">${langLabel}<button class="md-copy-btn" onclick="(function(btn){navigator.clipboard.writeText(btn.closest('.md-code-block').querySelector('code').textContent);btn.textContent='Copied!';setTimeout(function(){btn.textContent='Copy'},1500)})(this)">Copy</button><pre><code class="hljs${language ? ` language-${language}` : ""}">${highlighted}</code></pre></div>`;
};

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

const markedInstance = new Marked({ renderer, breaks: true, gfm: true });

export function Markdown(props: { content: string }) {
  const html = createMemo(() => {
    try {
      return markedInstance.parse(props.content || "") as string;
    } catch {
      return props.content;
    }
  });

  return <div class="md-render" innerHTML={html()} />;
}

// Inject styles once
if (!document.getElementById("md-styles")) {
  const style = document.createElement("style");
  style.id = "md-styles";
  style.textContent = `
    .md-render {
      font-size: 14px;
      line-height: 1.6;
      color: var(--text, #ebebf0);
      word-break: break-word;
    }
    .md-render > *:first-child { margin-top: 0; }
    .md-render > *:last-child { margin-bottom: 0; }

    /* Paragraphs */
    .md-render p {
      margin: 0.5em 0;
    }

    /* Headers */
    .md-render h1, .md-render h2, .md-render h3,
    .md-render h4, .md-render h5, .md-render h6 {
      margin: 1em 0 0.5em;
      font-weight: 600;
      line-height: 1.3;
      color: var(--text, #ebebf0);
    }
    .md-render h1 { font-size: 1.4em; }
    .md-render h2 { font-size: 1.25em; }
    .md-render h3 { font-size: 1.1em; }
    .md-render h4, .md-render h5, .md-render h6 { font-size: 1em; }

    /* Inline code */
    .md-render code:not(.hljs) {
      background: rgba(255, 255, 255, 0.08);
      padding: 2px 6px;
      border-radius: 4px;
      font-family: var(--font-mono);
      font-size: 0.88em;
      color: #dda0f7;
    }

    /* Code blocks */
    .md-code-block {
      position: relative;
      margin: 0.75em 0;
      border-radius: 8px;
      background: var(--bg-base, #131316);
      border: 1px solid rgba(255, 255, 255, 0.06);
      overflow: hidden;
    }
    .md-code-block pre {
      margin: 0;
      padding: 14px 16px;
      overflow-x: auto;
    }
    .md-code-block code.hljs {
      background: transparent;
      padding: 0;
      font-family: var(--font-mono);
      font-size: 13px;
      line-height: 1.5;
      color: #ebebf0;
    }
    .md-code-lang {
      position: absolute;
      top: 6px;
      left: 12px;
      font-size: 11px;
      color: var(--text-tertiary, #6b6b80);
      font-family: "SF Mono", "Fira Code", monospace;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
    .md-copy-btn {
      position: absolute;
      top: 6px;
      right: 8px;
      font-size: 11px;
      padding: 3px 10px;
      border-radius: 4px;
      background: rgba(255, 255, 255, 0.06);
      color: var(--text-tertiary, #6b6b80);
      border: none;
      cursor: pointer;
      opacity: 0;
      transition: opacity 0.15s, background 0.15s, color 0.15s;
    }
    .md-code-block:hover .md-copy-btn { opacity: 1; }
    .md-copy-btn:hover {
      background: rgba(255, 255, 255, 0.12);
      color: var(--text-secondary, #a0a0b0);
    }

    /* Lists */
    .md-render ul, .md-render ol {
      margin: 0.5em 0;
      padding-left: 1.5em;
    }
    .md-render li { margin: 0.2em 0; }
    .md-render li > p { margin: 0.25em 0; }

    /* Blockquotes */
    .md-render blockquote {
      margin: 0.75em 0;
      padding: 4px 16px;
      border-left: 3px solid var(--primary, #6680f2);
      color: var(--text-secondary, #a0a0b0);
      background: rgba(102, 128, 242, 0.04);
      border-radius: 0 4px 4px 0;
    }
    .md-render blockquote p { margin: 0.3em 0; }

    /* Tables */
    .md-render table {
      border-collapse: collapse;
      margin: 0.75em 0;
      width: 100%;
      font-size: 13px;
    }
    .md-render th, .md-render td {
      padding: 8px 12px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      text-align: left;
    }
    .md-render th {
      background: rgba(255, 255, 255, 0.04);
      font-weight: 600;
      color: var(--text, #ebebf0);
    }
    .md-render td {
      color: var(--text-secondary, #a0a0b0);
    }

    /* Links */
    .md-render a {
      color: var(--primary, #6680f2);
      text-decoration: none;
    }
    .md-render a:hover {
      text-decoration: underline;
      opacity: 0.85;
    }

    /* Horizontal rules */
    .md-render hr {
      border: none;
      border-top: 1px solid rgba(255, 255, 255, 0.08);
      margin: 1em 0;
    }

    /* Strong and emphasis */
    .md-render strong { color: var(--text, #ebebf0); font-weight: 600; }

    /* highlight.js token colors (dark theme) */
    .hljs-keyword, .hljs-selector-tag, .hljs-type { color: #c678dd; }
    .hljs-string, .hljs-addition { color: #98c379; }
    .hljs-number, .hljs-literal { color: #d19a66; }
    .hljs-comment, .hljs-deletion { color: #5c6370; font-style: italic; }
    .hljs-built_in, .hljs-builtin-name { color: #e6c07b; }
    .hljs-function .hljs-title, .hljs-title.function_ { color: #61afef; }
    .hljs-attr, .hljs-attribute { color: #d19a66; }
    .hljs-variable, .hljs-template-variable { color: #e06c75; }
    .hljs-params { color: #abb2bf; }
    .hljs-meta { color: #61afef; }
    .hljs-regexp { color: #56b6c2; }
    .hljs-tag { color: #e06c75; }
    .hljs-name { color: #e06c75; }
    .hljs-selector-id, .hljs-selector-class { color: #e6c07b; }
    .hljs-symbol, .hljs-bullet { color: #56b6c2; }
    .hljs-link { color: #61afef; text-decoration: underline; }
    .hljs-punctuation { color: #abb2bf; }
    .hljs-property { color: #e06c75; }
    .hljs-title.class_ { color: #e6c07b; }
  `;
  document.head.appendChild(style);
}
