/** Zed-styled popover CSS for the WorktreeSwitcher (bg-elevated, 1px hairlines,
 *  instant hovers, mono branch names). Keyframes come from global.css. */

export const SWITCHER_CSS = `
  .ws-backdrop { position: fixed; inset: 0; z-index: 99; }
  .ws-pop {
    position: fixed; z-index: 100; width: 320px;
    display: flex; flex-direction: column;
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
    animation: dropdown-in 0.14s var(--ease-out);
    overflow: hidden;
  }
  .ws-input {
    margin: 8px 8px 6px; padding: 6px 8px; flex-shrink: 0;
    font-family: var(--font-mono); font-size: 12px;
    background: var(--bg-muted); border: 1px solid var(--border);
    border-radius: var(--radius-sm); color: var(--text); outline: none;
  }
  .ws-input:focus { border-color: var(--border-glow); box-shadow: 0 0 0 2px var(--primary-glow); }
  .ws-list { overflow-y: auto; padding: 0 4px 4px; min-height: 0; }
  .ws-section {
    padding: 8px 6px 3px;
    font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em;
    color: var(--text-tertiary);
  }
  .ws-row {
    display: flex; align-items: center; gap: 7px; width: 100%;
    padding: 5px 6px; border-radius: var(--radius-sm);
    font-size: 12px; color: var(--text-secondary); text-align: left;
    cursor: pointer;
  }
  .ws-row:hover { background: var(--bg-accent); color: var(--text); }
  .ws-row--active { background: var(--bg-hover); color: var(--text); }
  .ws-row:disabled { opacity: 0.55; cursor: default; }
  .ws-ico { width: 12px; height: 12px; flex-shrink: 0; color: var(--text-tertiary); }
  .ws-row:hover .ws-ico { color: var(--primary); }
  .ws-mono { font-family: var(--font-mono); font-size: 11px; }
  .ws-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ws-remote { color: var(--text-tertiary); }
  .ws-dim {
    margin-left: auto; padding-left: 8px;
    font-size: 10.5px; color: var(--text-tertiary); flex-shrink: 0;
  }
  .ws-badge {
    font-size: 8.5px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em;
    color: var(--text-tertiary); background: var(--bg-muted);
    border: 1px solid var(--border-variant); padding: 0 4px; border-radius: var(--radius-pill);
    flex-shrink: 0;
  }
  .ws-dirty { width: 6px; height: 6px; border-radius: 50%; background: var(--amber); flex-shrink: 0; }
  .ws-count {
    font-family: var(--font-mono); font-size: 10px; font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .ws-ahead { color: var(--green); }
  .ws-behind { color: var(--orange); }
  .ws-trash {
    display: flex; align-items: center; justify-content: center;
    width: 18px; height: 18px; margin-left: 2px; border-radius: var(--radius-sm);
    color: var(--text-tertiary); opacity: 0; flex-shrink: 0;
  }
  .ws-row:hover .ws-trash { opacity: 0.7; }
  .ws-trash:hover { opacity: 1; color: var(--red); background: var(--bg-hover); }
  .ws-row--create { color: var(--text); }
  .ws-row--create .ws-ico, .ws-row--create:hover .ws-ico { color: var(--green); }
  .ws-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ws-text .ws-query { color: var(--primary); }
  .ws-status { padding: 6px 8px; font-size: 11px; color: var(--text-tertiary); }
  .ws-error {
    margin: 4px 6px; padding: 5px 8px; font-size: 11px; line-height: 1.45;
    color: var(--red); background: rgba(var(--red-rgb), 0.06);
    border: 1px solid rgba(var(--red-rgb), 0.18); border-radius: var(--radius-sm);
    user-select: text; -webkit-user-select: text; word-break: break-word;
  }
  .ws-footer {
    display: flex; align-items: center; justify-content: space-between; gap: 8px;
    padding: 6px 8px; border-top: 1px solid var(--border); flex-shrink: 0;
  }
  .ws-fetch {
    display: inline-flex; align-items: center; gap: 6px;
    font-size: 11.5px; font-weight: 600; padding: 3px 10px;
    color: var(--text-secondary); border: 1px solid var(--border); border-radius: var(--radius-sm);
  }
  .ws-fetch:hover { background: var(--bg-accent); color: var(--text); }
  .ws-fetch:disabled { opacity: 0.6; cursor: default; }
  .ws-hint { font-size: 10.5px; color: var(--text-tertiary); }
  .ws-spin {
    width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0;
    border: 1.5px solid var(--border-strong); border-top-color: var(--primary);
    animation: spin 0.7s linear infinite;
  }
`;
