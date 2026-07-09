/* Feature-tree CSS (FZ-1): collapsible group headers + the nested indent rail.
 * Injected once (id-guarded) rather than per-GroupNode, since GroupNode recurses.
 * Feature-row styling stays in FeatureRow; keyframes live in global.css. */

const STYLE_ID = "sidebar-tree-styles";

export function injectTreeStyles(): void {
  if (typeof document === "undefined" || document.getElementById(STYLE_ID)) return;
  const style = document.createElement("style");
  style.id = STYLE_ID;
  style.textContent = `
.ftg { display: flex; flex-direction: column; }
.ftg-header {
  display: flex; align-items: center; gap: 5px;
  width: calc(100% - 8px);
  margin: 1px var(--space-1);
  padding: 4px var(--space-3) 4px 6px;
  border-radius: var(--radius-sm);
  text-align: left;
}
.ftg-header:hover { background: var(--bg-hover); }
.ftg-chevron { flex-shrink: 0; color: var(--text-tertiary); transition: transform 0.16s var(--ease-out); }
.ftg-chevron--open { transform: rotate(90deg); }
.ftg-name {
  flex: 1; min-width: 0;
  font-size: 11px; font-weight: 600; letter-spacing: 0.01em;
  color: var(--text-secondary);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.ftg--ungrouped .ftg-name {
  color: var(--text-tertiary); font-weight: 500;
  text-transform: uppercase; letter-spacing: 0.06em; font-size: 10px;
}
.ftg-count {
  flex-shrink: 0;
  font-size: 9px; font-family: var(--font-mono);
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
  background: var(--bg-muted);
  padding: 0 5px; border-radius: 3px; line-height: 1.6;
}
.ftg-body { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.18s var(--ease-out); }
.ftg-body--open { grid-template-rows: 1fr; }
.ftg-body-inner { overflow: hidden; margin-left: 13px; border-left: 1px solid var(--border-variant); }
`;
  document.head.appendChild(style);
}
