/* Session-pane stylesheet — pane chrome (tabs/composer), the message stream, and
 * the agent-stream blocks (tool cards, thinking, typing, approval). Injected once
 * (id-guarded) since SessionPane, MessageStream and blocks.tsx share these class
 * names. Keyframes live in global.css; this file only consumes them. */

const STYLE_ID = "session-styles";

export function injectSessionStyles(): void {
  if (typeof document === "undefined" || document.getElementById(STYLE_ID)) return;
  const style = document.createElement("style");
  style.id = STYLE_ID;
  style.textContent = CSS;
  document.head.appendChild(style);
}

const CSS = `
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
  transition: background 0.15s, color 0.15s;
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
  border-radius: 3px;
  color: var(--text-tertiary);
  opacity: 0;
  transition: opacity 0.12s, background 0.12s, color 0.12s;
}
.sp-tab:hover .sp-tab-close, .sp-tab--active .sp-tab-close { opacity: 0.7; }
.sp-tab-close:hover { opacity: 1; background: var(--bg-accent); color: var(--text); }

.sp-tab-actions { display: flex; align-items: center; gap: 2px; flex-shrink: 0; padding-bottom: 5px; }
.sp-icon-btn {
  display: flex; align-items: center; justify-content: center;
  width: 22px; height: 22px;
  border-radius: var(--radius-sm);
  color: var(--text-tertiary);
  transition: background 0.12s, color 0.12s;
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
  transition: background 0.1s;
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
.sp-menu-input:focus { border-color: var(--primary); box-shadow: 0 0 0 2px var(--primary-glow); }

/* ══ Body ══ */
.sp-body { flex: 1; display: flex; flex-direction: column; min-height: 0; }
.sp-empty { margin: auto; text-align: center; max-width: 260px; padding: var(--space-6); animation: fade-slide-up 0.22s var(--ease-out) both; }
.sp-empty-title { font-size: 13px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
.sp-empty-sub { font-size: 11.5px; line-height: 1.5; color: var(--text-tertiary); }
.sp-mono { font-family: var(--font-mono); color: var(--primary); }

/* ══ Message stream ══ */
.sp-stream {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4) var(--space-3) var(--space-5);
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.sp-stream-hint {
  margin: auto;
  max-width: 240px;
  text-align: center;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--text-tertiary);
}

/* User — right-aligned tinted bubble */
.msg-user-bubble {
  max-width: 85%;
  margin-left: auto;
  padding: 9px 13px;
  border-radius: var(--radius-lg);
  font-size: 13.5px;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
  background: rgba(var(--primary-rgb), 0.08);
  border: 1px solid rgba(var(--primary-rgb), 0.14);
  color: var(--text);
  user-select: text; -webkit-user-select: text;
  animation: msg-user-in 0.18s var(--ease-out) both;
}

/* Assistant — full-width, bubble-less */
.msg-assistant {
  font-size: 13.5px;
  line-height: 1.6;
  color: var(--text);
  user-select: text; -webkit-user-select: text;
  animation: msg-assistant-in 0.18s var(--ease-out) both;
}
.msg-assistant .md-render { font-size: 13.5px; }

/* System — centered pill */
.msg-system-row { display: flex; justify-content: center; }
.msg-system-pill {
  text-align: center;
  background: var(--bg-muted);
  border: 1px solid var(--border);
  font-size: 11px;
  color: var(--text-secondary);
  border-radius: var(--radius-pill);
  padding: 3px 11px;
  max-width: 90%;
  word-break: break-word;
  animation: fade-slide-up 0.18s var(--ease-out) both;
}
.msg-system-pill--warn  { color: var(--amber); background: rgba(var(--amber-rgb), 0.10); border-color: rgba(var(--amber-rgb), 0.35); }
.msg-system-pill--error { color: var(--red);   background: rgba(var(--red-rgb), 0.10);   border-color: rgba(var(--red-rgb), 0.35); }

/* Usage footer */
.sp-usage {
  font-size: 9px;
  font-family: var(--font-mono);
  color: var(--text-tertiary);
  opacity: 0.75;
  letter-spacing: 0.02em;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

/* ══ Tool cards ══ */
.tc {
  border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--border);
  overflow: hidden;
  transition: border-color 0.2s, box-shadow 0.2s;
  animation: tool-card-in 0.18s var(--ease-out) both;
}
.tc--active { border-color: rgba(var(--amber-rgb), 0.2); box-shadow: 0 0 12px -4px rgba(var(--amber-rgb), 0.08); }
.tc--error  { border-color: rgba(var(--red-rgb), 0.25); }
.tc--done   { border-color: rgba(var(--green-rgb), 0.12); }
.tc-header {
  display: flex; align-items: center; gap: 7px;
  width: 100%;
  padding: 7px 10px;
  text-align: left;
  transition: background 0.1s;
}
.tc-header:hover { background: rgba(255, 255, 255, 0.025); }
.sp-chevron { flex-shrink: 0; color: var(--text-tertiary); transition: transform 0.18s ease; }
.sp-chevron--open { transform: rotate(90deg); }
.tc-name { font-size: 12px; font-weight: 600; color: var(--text); letter-spacing: -0.01em; flex-shrink: 0; }
.tc--active .tc-name { color: var(--amber); }
.tc--done .tc-name { color: var(--text); }
.tc--error .tc-name { color: var(--red); }
.tc-summary {
  flex: 1; min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  font-family: var(--font-mono); font-size: 11px; color: var(--text-tertiary); opacity: 0.8;
}
.tc-pulse { width: 6px; height: 6px; border-radius: 50%; background: var(--amber); flex-shrink: 0; animation: dot-pulse 1.5s ease-in-out infinite; }
.tc-status { font-size: 10px; font-weight: 500; font-family: var(--font-mono); color: var(--text-tertiary); letter-spacing: 0.02em; flex-shrink: 0; }
.tc--active .tc-status { color: var(--amber); }
.tc--done .tc-status { color: var(--green); }
.tc-status--error, .tc--error .tc-status { color: var(--red); }

.tc-body { display: grid; grid-template-rows: 0fr; transition: grid-template-rows 0.2s ease; }
.tc-body--open { grid-template-rows: 1fr; }
.tc-body-inner { overflow: hidden; }
.tc-section { padding: 8px 10px; border-top: 1px solid var(--border); }
.tc-section-label { font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-tertiary); margin-bottom: 4px; }
.tc-code {
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 1.55;
  color: var(--text-secondary);
  background: var(--bg-base);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  overflow-x: auto;
  max-height: 280px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-word;
  user-select: text; -webkit-user-select: text;
}
.tc-code--error { color: var(--red); }

/* ══ Thinking block ══ */
.tb {
  border-radius: var(--radius-md);
  background: rgba(var(--purple-rgb), 0.03);
  border: 1px solid rgba(var(--purple-rgb), 0.1);
  overflow: hidden;
  transition: border-color 0.2s;
}
.tb--streaming { border-color: rgba(var(--purple-rgb), 0.22); }
.tb-label { font-size: 12px; font-weight: 500; color: var(--purple); flex-shrink: 0; }
.tb-preview { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11.5px; color: var(--text-tertiary); font-style: italic; opacity: 0.7; }
.tb-dots { display: inline-flex; gap: 3px; align-items: center; }
.tb-dots span { width: 3.5px; height: 3.5px; border-radius: 50%; background: var(--purple); animation: thinking-dot 1.4s infinite both; }
.tb-dots span:nth-child(2) { animation-delay: 0.16s; }
.tb-dots span:nth-child(3) { animation-delay: 0.32s; }
.tb-full {
  padding: 8px 10px;
  border-top: 1px solid rgba(var(--purple-rgb), 0.1);
  font-size: 11.5px; line-height: 1.55;
  color: var(--text-secondary); font-style: italic;
  white-space: pre-wrap; word-break: break-word;
  user-select: text; -webkit-user-select: text;
}

/* ══ Typing indicator — three shimmer gradient lines ══ */
.typing { display: flex; flex-direction: column; gap: 6px; padding: 2px 0; }
.typing-shimmer-line {
  height: 3px;
  border-radius: 2px;
  background: linear-gradient(90deg,
    rgba(var(--primary-rgb), 0.08) 0%,
    rgba(var(--primary-rgb), 0.25) 40%,
    rgba(var(--primary-rgb), 0.08) 80%);
  background-size: 200% 100%;
  animation: shimmer-flow 1.5s ease-in-out infinite;
}
.typing-shimmer-line.short { width: 60%; }

/* ══ Approval card ══ */
.approval-card {
  background: rgba(var(--amber-rgb), 0.04);
  border: 1px solid rgba(var(--amber-rgb), 0.4);
  border-radius: var(--radius-md);
  padding: 12px 13px;
  display: flex;
  flex-direction: column;
  gap: 9px;
  animation: approval-enter 0.3s var(--ease-out) both, approval-pulse 2s ease-in-out 0.3s infinite;
}
.ac-header { display: flex; align-items: center; gap: 7px; }
.ac-header svg { color: var(--amber); flex-shrink: 0; }
.ac-title { font-size: 12.5px; font-weight: 600; color: var(--amber); }
.ac-tool {
  margin-left: auto;
  font-family: var(--font-mono); font-size: 10px;
  color: var(--text-secondary);
  background: var(--bg-accent);
  padding: 2px 7px; border-radius: var(--radius-pill);
}
.ac-input {
  font-family: var(--font-mono); font-size: 11px; line-height: 1.5;
  color: var(--text-secondary);
  background: var(--bg-base);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  max-height: 180px; overflow-y: auto;
  white-space: pre-wrap; word-break: break-word;
  user-select: text; -webkit-user-select: text;
}
.ac-actions { display: flex; justify-content: flex-end; gap: 8px; }
.ac-deny, .ac-approve {
  padding: 5px 14px;
  font-size: 12px; font-weight: 600;
  border-radius: var(--radius-sm);
  transition: filter 0.15s, background 0.15s, transform 0.1s;
}
.ac-deny { background: var(--bg-muted); border: 1px solid var(--border); color: var(--text-secondary); }
.ac-deny:hover { background: var(--bg-accent); color: var(--text); }
.ac-approve { background: var(--green); color: #fff; }
.ac-approve:hover { filter: brightness(1.1); }
.ac-deny:active, .ac-approve:active { transform: scale(0.97); }
.ac-deny:disabled, .ac-approve:disabled { opacity: 0.5; cursor: default; }

/* ══ Composer ══ */
.sp-composer-wrap { padding: 8px 12px 12px; flex-shrink: 0; }
.sp-composer-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-xl);
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  transition: border-color 0.3s, box-shadow 0.3s;
}
.sp-composer-card:focus-within {
  border-color: var(--border-glow);
  box-shadow: 0 0 0 2px var(--primary-glow), 0 4px 16px rgba(0, 0, 0, 0.15);
}
.sp-composer-card.generating { animation: composer-gen-pulse 2s ease-in-out infinite; }
.sp-composer-card.disabled { opacity: 0.6; }
.sp-input {
  flex: 1;
  background: none; border: none; outline: none; box-shadow: none;
  color: var(--text);
  font-size: 13.5px;
  line-height: 1.45;
  resize: none;
  padding: 2px 0;
  min-height: 20px;
  max-height: 168px;
  font-family: var(--font-body);
}
.sp-input::placeholder { color: var(--text-tertiary); }
.sp-input:disabled { cursor: default; }
.sp-composer-meta { display: flex; align-items: center; gap: 8px; }
.sp-model-tag {
  font-family: var(--font-mono); font-size: 10px;
  color: var(--text-tertiary);
  background: var(--bg-muted);
  border: 1px solid var(--border);
  padding: 2px 8px; border-radius: var(--radius-pill);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 160px;
}
.sp-composer-spacer { flex: 1; }
.sp-send {
  width: 30px; height: 30px; border-radius: 50%;
  background: var(--primary); color: #fff;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0;
  transition: background 0.25s, filter 0.15s, transform 0.1s;
}
.sp-send:hover { filter: brightness(1.1); transform: scale(1.04); }
.sp-send:active { transform: scale(0.96); }
.sp-send:disabled { opacity: 0.4; cursor: default; transform: none; filter: none; }
.sp-send.stop { background: var(--amber); }
.sp-send.stop:hover { filter: brightness(1.08); }
`;
