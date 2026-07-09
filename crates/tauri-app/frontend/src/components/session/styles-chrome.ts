/* Session-pane chrome CSS: pane shell, tab strip, new-session model dropdown, the
 * slim session header, and the body/empty states. Consumed by styles.ts (injected
 * once). Keyframes live in global.css; this file only references them. */

export const CHROME_CSS = `
/* ══ Pane shell ══ */
.session-pane {
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  min-height: 0;
  background: var(--bg-surface);
  border-left: 1px solid var(--border);
}

/* ══ Tabs strip ══ */
.sp-tabs {
  display: flex;
  align-items: stretch;
  gap: 2px;
  height: 34px;
  padding: 5px 6px 0;
  background: var(--bg-muted);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.sp-tabs-scroll {
  display: flex;
  align-items: stretch;
  gap: 2px;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}
.sp-tabs-scroll::-webkit-scrollbar { display: none; }
.sp-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  max-width: 160px;
  padding: 5px 8px 5px 10px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  white-space: nowrap;
  flex-shrink: 0;
}
.sp-tab:hover { background: var(--bg-hover); }
.sp-tab--active {
  background: var(--bg-surface);
  color: var(--text);
  border: 1px solid var(--border-strong);
  border-bottom: 1px solid var(--bg-surface);
  margin-bottom: -1px;
}
.sp-tab-title { overflow: hidden; text-overflow: ellipsis; }
.sp-tab-close {
  display: flex; align-items: center; justify-content: center;
  width: 14px; height: 14px;
  border-radius: var(--radius-sm);
  color: var(--text-tertiary);
  opacity: 0;
}
.sp-tab:hover .sp-tab-close, .sp-tab--active .sp-tab-close { opacity: 0.7; }
.sp-tab-close:hover { opacity: 1; background: var(--bg-accent); color: var(--text); }

.sp-tab-actions { display: flex; align-items: center; gap: 2px; flex-shrink: 0; padding-bottom: 5px; }
.sp-icon-btn {
  display: flex; align-items: center; justify-content: center;
  width: 22px; height: 22px;
  border-radius: var(--radius-sm);
  color: var(--text-tertiary);
}
.sp-icon-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }
.sp-icon-btn:disabled { opacity: 0.35; cursor: default; }
.sp-icon-btn:disabled:hover { background: none; color: var(--text-tertiary); }

/* ── New-session model dropdown ── */
.sp-new-wrap { position: relative; display: flex; }
.sp-menu-backdrop { position: fixed; inset: 0; z-index: 99; }
.sp-model-menu {
  position: absolute;
  top: calc(100% + 4px);
  right: 0;
  z-index: 100;
  min-width: 190px;
  padding: 4px;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.03);
  animation: dropdown-in 0.14s var(--ease-out);
}
.sp-menu-label {
  font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em;
  color: var(--text-tertiary);
  padding: 6px 8px 4px;
}
.sp-menu-item {
  display: flex; flex-direction: column; gap: 1px;
  width: 100%;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  text-align: left;
}
.sp-menu-item:hover { background: var(--bg-accent); }
.sp-menu-item-name { font-size: 12px; font-weight: 500; color: var(--text); }
.sp-menu-item-desc { font-size: 10px; color: var(--text-tertiary); }
.sp-menu-custom { padding: 4px; border-top: 1px solid var(--border); margin-top: 2px; }
.sp-menu-input {
  width: 100%;
  font-size: 12px;
  font-family: var(--font-mono);
  background: var(--bg-muted);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 5px 8px;
  color: var(--text);
  outline: none;
}
.sp-menu-input:focus { border-color: var(--border-glow); box-shadow: 0 0 0 2px var(--primary-glow); }

/* ══ Session header — slim model / mode / run-state bar below the tabs ══ */
.sp-header {
  display: flex; align-items: center; gap: 7px;
  height: 26px; padding: 0 12px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  animation: fade-slide-down 0.16s var(--ease-out) both;
}
.sp-h-icon { color: var(--text-tertiary); flex-shrink: 0; }
.sp-h-model {
  font-family: var(--font-mono); font-size: 10.5px; font-weight: 500;
  color: var(--text-secondary);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 130px;
}
.sp-h-dot-sep { width: 3px; height: 3px; border-radius: 50%; background: var(--text-tertiary); opacity: 0.6; flex-shrink: 0; }
.sp-h-mode { font-size: 10.5px; color: var(--text-tertiary); flex-shrink: 0; }
.sp-h-spacer { flex: 1; }
.sp-h-state {
  display: inline-flex; align-items: center; gap: 5px;
  font-size: 10px; font-weight: 500; font-family: var(--font-mono);
  letter-spacing: 0.02em; flex-shrink: 0;
}
.sp-h-pulse { width: 5px; height: 5px; border-radius: 50%; animation: dot-pulse 1.5s ease-in-out infinite; }
.sp-h-state--gen { color: var(--sky); }
.sp-h-state--gen .sp-h-pulse { background: var(--sky); }
.sp-h-state--start { color: var(--amber); }
.sp-h-state--start .sp-h-pulse { background: var(--amber); }
.sp-h-state--err { color: var(--red); }
.sp-h-state--ready { color: var(--text-tertiary); opacity: 0.7; }

/* ══ Body ══ */
.sp-body { flex: 1; display: flex; flex-direction: column; min-height: 0; }
.sp-empty { margin: auto; text-align: center; max-width: 260px; padding: var(--space-6); animation: fade-slide-up 0.22s var(--ease-out) both; }
.sp-empty-title { font-size: 13px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
.sp-empty-sub { font-size: 11.5px; line-height: 1.5; color: var(--text-tertiary); }
.sp-mono { font-family: var(--font-mono); color: var(--primary); }
`;
