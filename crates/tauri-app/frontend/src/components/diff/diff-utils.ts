/* Pure helpers for diff review: feature-group ordering, unique-file counts,
 * status glyph/tint mapping, extension→hljs language, and per-line highlighted
 * render rows. Truncation is the backend's single decision (FileDiff.truncated);
 * this module never re-caps. */

import hljs from "highlight.js";
import type { FeatureDiffGroup, FileDiff } from "../../types";

// ── Feature groups ────────────────────────────────────────────────────────────

/** The synthetic bucket of files that matched no feature — a named backend
 *  flag (CONTRACT-4), not a slug-string sniff. */
export function isUnmapped(group: FeatureDiffGroup): boolean {
  return group.unmapped;
}

/** Drop empty groups; keep given order but force the unmapped group last. */
export function orderedGroups(groups: FeatureDiffGroup[]): FeatureDiffGroup[] {
  const nonEmpty = groups.filter((g) => g.files.length > 0);
  const mapped = nonEmpty.filter((g) => !isUnmapped(g));
  const unmapped = nonEmpty.filter(isUnmapped);
  return [...mapped, ...unmapped];
}

export interface Counts {
  additions: number;
  deletions: number;
}

/** Sum a single group's files (paths are unique within a group). */
export function groupCounts(group: FeatureDiffGroup): Counts {
  let additions = 0;
  let deletions = 0;
  for (const f of group.files) {
    additions += f.additions;
    deletions += f.deletions;
  }
  return { additions, deletions };
}

/** Sum every unique file once — a shared file appears under several groups. */
export function totalCounts(groups: FeatureDiffGroup[]): Counts {
  const seen = new Map<string, FileDiff>();
  for (const g of groups) {
    for (const f of g.files) {
      if (!seen.has(f.path)) seen.set(f.path, f);
    }
  }
  let additions = 0;
  let deletions = 0;
  for (const f of seen.values()) {
    additions += f.additions;
    deletions += f.deletions;
  }
  return { additions, deletions };
}

// ── File status ───────────────────────────────────────────────────────────────

interface StatusMeta {
  letter: string;
  cls: "modified" | "added" | "deleted" | "renamed" | "untracked" | "unknown";
  label: string;
}

// All five backend statuses mapped exhaustively; each has its own chip tint.
const STATUS: Record<string, StatusMeta> = {
  modified: { letter: "M", cls: "modified", label: "Modified" },
  added: { letter: "A", cls: "added", label: "Added" },
  deleted: { letter: "D", cls: "deleted", label: "Deleted" },
  renamed: { letter: "R", cls: "renamed", label: "Renamed" },
  untracked: { letter: "U", cls: "untracked", label: "Untracked" },
};

/** A status the backend didn't emit resolves to an explicit neutral "unknown"
 *  chip — never silently painted as "modified". */
export function statusMeta(status: string): StatusMeta {
  return STATUS[status] ?? { letter: "?", cls: "unknown", label: "Unknown" };
}

export function splitPath(path: string): { dir: string; base: string } {
  const i = path.lastIndexOf("/");
  return i < 0 ? { dir: "", base: path } : { dir: path.slice(0, i + 1), base: path.slice(i + 1) };
}

// ── Syntax highlighting ───────────────────────────────────────────────────────

const EXT_LANG: Record<string, string> = {
  ts: "typescript", tsx: "typescript", mts: "typescript", cts: "typescript",
  js: "javascript", jsx: "javascript", mjs: "javascript", cjs: "javascript",
  rs: "rust", py: "python", rb: "ruby", go: "go", java: "java", kt: "kotlin",
  swift: "swift", c: "c", h: "c", cc: "cpp", cpp: "cpp", cxx: "cpp", hpp: "cpp",
  cs: "csharp", php: "php", scala: "scala", sh: "bash", bash: "bash", zsh: "bash",
  json: "json", yaml: "yaml", yml: "yaml", toml: "ini", ini: "ini",
  md: "markdown", markdown: "markdown", html: "xml", xml: "xml", vue: "xml",
  css: "css", scss: "scss", less: "less", sql: "sql", graphql: "graphql", gql: "graphql",
  lua: "lua", dart: "dart", ex: "elixir", exs: "elixir", erl: "erlang", hs: "haskell",
  ml: "ocaml", clj: "clojure", r: "r", pl: "perl", proto: "protobuf", dockerfile: "dockerfile",
};

export function langForPath(path: string): string | undefined {
  const base = splitPath(path).base.toLowerCase();
  if (base === "dockerfile") return "dockerfile";
  const ext = base.includes(".") ? base.slice(base.lastIndexOf(".") + 1) : "";
  return EXT_LANG[ext];
}

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function highlight(content: string, lang: string | undefined): string {
  if (content.length === 0) return "";
  if (lang) {
    try {
      if (hljs.getLanguage(lang)) return hljs.highlight(content, { language: lang }).value;
    } catch {
      /* fall through to escaped */
    }
  }
  return escapeHtml(content);
}

// ── Render rows (flattened hunks; no re-truncation) ───────────────────────────

export type DiffRow =
  | { t: "header"; key: string; header: string }
  | { t: "line"; key: string; origin: "+" | "-" | " "; oldNo: number | null; newNo: number | null; html: string };

/** Flatten a file's hunks into render rows. Renders every line the backend
 *  sent — the backend already capped at its line limit (FileDiff.truncated). */
export function buildRows(file: FileDiff, lang: string | undefined): DiffRow[] {
  const rows: DiffRow[] = [];
  for (let hi = 0; hi < file.hunks.length; hi++) {
    const hunk = file.hunks[hi];
    rows.push({ t: "header", key: `h${hi}`, header: hunk.header });
    for (let li = 0; li < hunk.lines.length; li++) {
      const line = hunk.lines[li];
      rows.push({
        t: "line",
        key: `${hi}:${li}`,
        origin: line.origin,
        oldNo: line.oldNo,
        newNo: line.newNo,
        html: highlight(line.content, lang),
      });
    }
  }
  return rows;
}

// ── One-time style injection (rows render many times) ─────────────────────────

export function injectDiffStyles(): void {
  if (document.getElementById("diff-styles")) return;
  const style = document.createElement("style");
  style.id = "diff-styles";
  style.textContent = DIFF_CSS;
  document.head.appendChild(style);
}

const DIFF_CSS = `
/* ── View shell ── */
.drv { flex: 1; display: flex; flex-direction: column; min-height: 0; }
.drv-header {
  position: sticky; top: 0; z-index: 2;
  display: flex; align-items: center; gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  background: var(--bg-base); border-bottom: 1px solid var(--border);
}
.drv-title { font-size: 12px; font-weight: 600; color: var(--text-secondary); letter-spacing: -0.2px; }
.drv-totals { display: flex; gap: 10px; font-family: var(--font-mono); font-size: 11px; font-variant-numeric: tabular-nums; }
.drv-total-add { color: var(--green); }
.drv-total-del { color: var(--red); }
.drv-spacer { flex: 1; }
.drv-refresh {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px; font-weight: 500; color: var(--text-secondary);
  padding: 4px 10px; border-radius: var(--radius-sm);
  transition: background 0.12s, color 0.12s;
}
.drv-refresh:hover { background: var(--bg-accent); color: var(--text); }
.drv-refresh svg { color: var(--text-tertiary); }
.drv-refresh:hover svg { color: var(--primary); }
.drv-refresh--busy svg { animation: spin 0.7s linear infinite; }
.drv-list { max-width: 920px; width: 100%; margin: 0 auto; padding: var(--space-4); }
.drv-empty-wrap { flex: 1; display: flex; }
.drv-empty { margin: auto; text-align: center; max-width: 320px; animation: fade-slide-up 0.22s var(--ease-out) both; }
.drv-empty-title { font-size: 15px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
.drv-empty-sub { font-size: 12px; line-height: 1.5; color: var(--text-tertiary); }

/* ── Feature group accordion ── */
.dfg {
  margin-bottom: var(--space-3);
  border: 1px solid var(--border); border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.02); overflow: hidden;
  animation: fade-slide-up 0.18s var(--ease-out) both;
}
.dfg--unmapped { background: rgba(255, 255, 255, 0.012); }
.dfg-head {
  display: flex; align-items: center; gap: var(--space-2);
  width: 100%; padding: 9px 12px; text-align: left; cursor: pointer;
  transition: background 0.12s;
}
.dfg-head:hover { background: var(--bg-hover); }
.dfg-chevron { flex-shrink: 0; color: var(--text-tertiary); transition: transform 0.18s ease; }
.dfg-chevron--open { transform: rotate(90deg); }
.dfg-name { font-size: 12px; font-weight: 600; color: var(--text); }
.dfg--unmapped .dfg-name { color: var(--text-tertiary); font-weight: 500; }
.dfg-shared { font-size: 9px; font-family: var(--font-mono); padding: 0 6px; line-height: 1.7; border-radius: var(--radius-pill); }
.dfg-spacer { flex: 1; }
.dfg-counts { display: flex; gap: 8px; font-family: var(--font-mono); font-size: 10px; font-variant-numeric: tabular-nums; }
.dfg-add { color: var(--green); }
.dfg-del { color: var(--red); }
.dfg-filecount { font-family: var(--font-mono); font-size: 10px; color: var(--text-tertiary); min-width: 44px; text-align: right; }
.dfg-body { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.25s ease; }
.dfg-body--open { grid-template-rows: 1fr; }
.dfg-body-inner { overflow: hidden; min-height: 0; border-top: 1px solid var(--border); }

/* ── File row ── */
.dfr { border-top: 1px solid var(--border); }
.dfr:first-child { border-top: none; }
.dfr-head {
  display: flex; align-items: center; gap: var(--space-2);
  width: 100%; padding: 6px 12px; text-align: left; cursor: pointer;
  transition: background 0.12s;
}
.dfr-head:hover { background: var(--bg-hover); }
.dfr-status {
  display: inline-flex; align-items: center; justify-content: center;
  width: 16px; height: 16px; border-radius: 4px; flex-shrink: 0;
  font-size: 9px; font-weight: 600; font-family: var(--font-mono);
}
.dfr-status--modified { background: rgba(var(--primary-rgb), 0.15); color: var(--primary); }
.dfr-status--added { background: rgba(var(--green-rgb), 0.15); color: var(--green); }
.dfr-status--deleted { background: rgba(var(--red-rgb), 0.15); color: var(--red); }
.dfr-status--renamed { background: rgba(245, 148, 60, 0.15); color: var(--orange); }
.dfr-status--untracked { background: rgba(120, 170, 255, 0.15); color: var(--sky); }
.dfr-status--unknown { background: var(--bg-muted); color: var(--text-tertiary); }
.dfr-path { flex: 1; min-width: 0; font-family: var(--font-mono); font-size: 11.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.dfr-dir { color: var(--text-tertiary); }
.dfr-base { color: var(--text); }
.dfr-counts { display: flex; gap: 8px; flex-shrink: 0; font-family: var(--font-mono); font-size: 10px; font-variant-numeric: tabular-nums; }
.dfr-add { color: var(--green); }
.dfr-del { color: var(--red); }
.dfr-chevron { flex-shrink: 0; color: var(--text-tertiary); opacity: 0; transition: opacity 0.12s, transform 0.18s ease; }
.dfr-head:hover .dfr-chevron { opacity: 0.6; }
.dfr-chevron--open { transform: rotate(90deg); opacity: 0.6; }
.dfr-body { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.2s ease; }
.dfr-body--open { grid-template-rows: 1fr; }
.dfr-body-inner { overflow: hidden; min-height: 0; }

/* ── Hunks & lines ── */
.dh { border-top: 1px solid var(--border); background: var(--bg-base); }
.dh-hunk-header {
  padding: 2px 10px; white-space: pre;
  font-family: var(--font-mono); font-size: 10.5px; color: var(--text-tertiary);
  background: var(--bg-muted); border-top: 1px solid var(--border);
}
.dh-hunk-header:first-child { border-top: none; }
.dh-line {
  display: grid; grid-template-columns: 38px 38px 15px 1fr; align-items: baseline;
  font-family: var(--font-mono); font-size: 11.5px; line-height: 1.5;
}
.dh-gutter {
  text-align: right; padding-right: 8px; user-select: none;
  font-size: 10px; color: var(--text-tertiary); opacity: 0.55; font-variant-numeric: tabular-nums;
}
.dh-prefix { text-align: center; user-select: none; color: var(--text-tertiary); }
.dh-content { white-space: pre-wrap; word-break: break-word; overflow-wrap: anywhere; padding-right: 12px; color: var(--hljs-code-text); }
.dh-line--add { background: rgba(var(--green-rgb), 0.08); }
.dh-line--add .dh-prefix { color: var(--green); }
.dh-line--remove { background: rgba(var(--red-rgb), 0.08); }
.dh-line--remove .dh-prefix { color: var(--red); }
.dh-trunc { display: flex; justify-content: center; padding: 8px; background: var(--bg-base); }
.dh-trunc-pill {
  font-family: var(--font-mono); font-size: 10px; color: var(--text-secondary);
  background: var(--bg-muted); border: 1px solid var(--border);
  border-radius: var(--radius-pill); padding: 3px 12px;
}
.dh-binary {
  padding: 10px 12px;
  font-family: var(--font-mono); font-size: 11px; font-style: italic;
  color: var(--text-tertiary);
  background: var(--bg-base); border-top: 1px solid var(--border);
}

/* hljs token colors route through the theme vars (same mapping as markdown) */
.dh-content .hljs-keyword, .dh-content .hljs-selector-tag, .dh-content .hljs-type { color: var(--hljs-keyword); }
.dh-content .hljs-string, .dh-content .hljs-addition { color: var(--hljs-string); }
.dh-content .hljs-number, .dh-content .hljs-literal { color: var(--hljs-number); }
.dh-content .hljs-comment, .dh-content .hljs-quote { color: var(--hljs-comment); font-style: italic; }
.dh-content .hljs-built_in, .dh-content .hljs-builtin-name { color: var(--hljs-builtin); }
.dh-content .hljs-function .hljs-title, .dh-content .hljs-title.function_ { color: var(--hljs-function); }
.dh-content .hljs-attr, .dh-content .hljs-attribute { color: var(--hljs-attr); }
.dh-content .hljs-variable, .dh-content .hljs-template-variable { color: var(--hljs-variable); }
.dh-content .hljs-params { color: var(--hljs-params); }
.dh-content .hljs-meta, .dh-content .hljs-doctag { color: var(--hljs-meta); }
.dh-content .hljs-regexp { color: var(--hljs-regexp); }
.dh-content .hljs-tag { color: var(--hljs-tag); }
.dh-content .hljs-selector-class, .dh-content .hljs-selector-id { color: var(--hljs-selector); }
.dh-content .hljs-symbol, .dh-content .hljs-bullet { color: var(--hljs-symbol); }
.dh-content .hljs-link { color: var(--hljs-link); }
.dh-content .hljs-property { color: var(--hljs-property); }
.dh-content .hljs-title.class_, .dh-content .hljs-class .hljs-title { color: var(--hljs-class); }
`;
