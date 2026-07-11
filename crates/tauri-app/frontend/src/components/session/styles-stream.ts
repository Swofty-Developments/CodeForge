/* Message-stream CSS: the scroll list, user/assistant/system messages, the usage
 * footer, and the agent-stream blocks (tool cards, thinking, typing indicator,
 * approval card). Consumed by styles.ts. Keyframes live in global.css. */

export const STREAM_CSS = `
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
  padding: 7px 11px;
  border-radius: var(--radius-md);
  font-size: 13px;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--bg-user-bubble);
  border: 1px solid var(--border);
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
  border: 1px solid var(--border-variant);
  font-size: 10.5px;
  color: var(--text-tertiary);
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
  border-radius: var(--radius-sm);
  background: var(--bg-muted);
  border: 1px solid var(--border);
  overflow: hidden;
  animation: tool-card-in 0.18s var(--ease-out) both;
}
.tc--active { border-color: rgba(var(--amber-rgb), 0.3); }
.tc--error  { border-color: rgba(var(--red-rgb), 0.3); }
.tc--done   { border-color: rgba(var(--green-rgb), 0.3); }
.tc-header {
  display: flex; align-items: center; gap: 7px;
  width: 100%;
  padding: 6px 10px;
  text-align: left;
}
.tc-header:hover { background: var(--bg-hover); }
.sp-chevron { flex-shrink: 0; color: var(--text-tertiary); transition: transform 0.18s ease; }
.sp-chevron--open { transform: rotate(90deg); }
.tc-name { font-size: 12px; font-weight: 600; color: var(--text); letter-spacing: -0.01em; flex-shrink: 0; }
.tc-name--unknown { color: var(--text-tertiary); font-weight: 500; font-style: italic; }
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
  border-radius: var(--radius-sm);
  background: var(--bg-muted);
  border: 1px solid var(--border-variant);
  overflow: hidden;
}
.tb--streaming { border-color: var(--border); }
.tb-label { font-size: 12px; font-weight: 500; color: var(--text-tertiary); font-style: italic; flex-shrink: 0; }
.tb-preview { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11.5px; color: var(--text-tertiary); font-style: italic; opacity: 0.7; }
.tb-dots { display: inline-flex; gap: 3px; align-items: center; }
.tb-dots span { width: 3.5px; height: 3.5px; border-radius: 50%; background: var(--text-tertiary); animation: thinking-dot 1.4s infinite both; }
.tb-dots span:nth-child(2) { animation-delay: 0.16s; }
.tb-dots span:nth-child(3) { animation-delay: 0.32s; }
.tb-full {
  padding: 8px 10px;
  border-top: 1px solid var(--border-variant);
  font-size: 11.5px; line-height: 1.55;
  color: var(--text-tertiary); font-style: italic;
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
.ac-tool--unknown { font-style: italic; color: var(--text-tertiary); }
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
}
.ac-deny { background: var(--bg-muted); border: 1px solid var(--border); color: var(--text-secondary); }
.ac-deny:hover { background: var(--bg-accent); color: var(--text); }
.ac-approve { background: var(--green); color: #22271f; }
.ac-approve:hover { filter: brightness(1.08); }
.ac-deny:disabled, .ac-approve:disabled { opacity: 0.5; cursor: default; }

/* ══ Question Card ══ */
.question-card .qc-questions {
  margin: var(--space-3) 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.qc-question {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.qc-question-text {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--text);
  line-height: 1.4;
}
.qc-multiselect-hint {
  font-size: 11px;
  color: var(--text-tertiary);
  font-style: italic;
}
.qc-options {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.qc-option {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
  padding: 10px 12px;
  background: var(--bg-muted);
  border: 1.5px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.15s var(--ease-out);
  text-align: left;
}
.qc-option:hover {
  background: var(--bg-accent);
  border-color: var(--border-strong);
}
.qc-option--selected {
  background: rgba(var(--primary-rgb), 0.12);
  border-color: var(--primary);
}
.qc-option--selected:hover {
  background: rgba(var(--primary-rgb), 0.18);
  border-color: var(--primary);
}
.qc-option-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text);
}
.qc-option--selected .qc-option-label {
  color: var(--primary);
}
.qc-option-desc {
  font-size: 11px;
  line-height: 1.4;
  color: var(--text-tertiary);
}
.qc-option--selected .qc-option-desc {
  color: var(--text-secondary);
}
`;
