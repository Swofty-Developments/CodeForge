/* Composer CSS: the composer card, attachment chips, drag-drop hint, the
 * slash-command popover, the autosize input, and the attach/send controls.
 * Consumed by styles.ts. Keyframes live in global.css. */

export const COMPOSER_CSS = `
/* ══ Composer ══ */
.sp-composer-wrap { padding: 8px 12px 12px; flex-shrink: 0; }
.sp-composer-card {
  position: relative;
  background: var(--bg-muted);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  transition: border-color var(--dur), box-shadow var(--dur);
}
.sp-composer-card:focus-within {
  border-color: var(--border-glow);
  box-shadow: 0 0 0 2px var(--primary-glow);
}
.sp-composer-card.generating { animation: composer-gen-pulse 2s ease-in-out infinite; }
.sp-composer-card.disabled { opacity: 0.6; }
.sp-composer-card.drag-over {
  border-color: var(--primary);
  box-shadow: 0 0 0 2px var(--primary-glow);
}

/* Attachment chips (above the input) */
.sp-attachments { display: flex; flex-wrap: wrap; gap: 5px; }
.sp-chip {
  display: inline-flex; align-items: center; gap: 5px;
  max-width: 200px;
  padding: 2px 4px 2px 8px;
  border-radius: var(--radius-pill);
  background: rgba(var(--primary-rgb), 0.1);
  border: 1px solid rgba(var(--primary-rgb), 0.25);
  color: var(--primary);
  font-size: 11px;
  animation: fade-slide-up 0.14s var(--ease-out) both;
}
.sp-chip svg { flex-shrink: 0; opacity: 0.85; }
.sp-chip-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: var(--font-mono); }
.sp-chip-x {
  display: flex; align-items: center; justify-content: center;
  width: 14px; height: 14px; flex-shrink: 0;
  border-radius: 50%; color: var(--primary); opacity: 0.7;
}
.sp-chip-x:hover { opacity: 1; background: rgba(var(--primary-rgb), 0.2); }

/* Drop hint overlay */
.sp-drop-hint {
  position: absolute; inset: 0;
  display: flex; align-items: center; justify-content: center;
  border-radius: var(--radius-lg);
  background: rgba(var(--primary-rgb), 0.08);
  color: var(--primary);
  font-size: 12px; font-weight: 600;
  pointer-events: none;
  animation: fade-in 0.12s ease-out both;
}

/* Input row (anchors the upward slash-command popover) */
.sp-input-row { position: relative; display: flex; }

/* Slash-command popover */
.slash-menu {
  position: absolute;
  bottom: calc(100% + 8px); left: -4px;
  z-index: 50;
  min-width: 220px; max-width: 300px;
  padding: 4px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.03);
  animation: dropdown-in 0.14s var(--ease-out);
}
.slash-menu-label {
  font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em;
  color: var(--text-tertiary);
  padding: 5px 8px 4px;
}
.slash-menu-list { max-height: 240px; overflow-y: auto; }
.slash-item {
  display: flex; align-items: baseline; gap: 1px;
  width: 100%; padding: 5px 8px; text-align: left;
  border-radius: var(--radius-sm);
}
.slash-item--sel { background: var(--bg-accent); }
.slash-item-slash { font-family: var(--font-mono); font-size: 12px; color: var(--text-tertiary); }
.slash-item-name { font-family: var(--font-mono); font-size: 12px; color: var(--text-secondary); }
.slash-item--sel .slash-item-name, .slash-item--sel .slash-item-slash { color: var(--primary); }
.slash-empty { padding: 8px; text-align: center; font-size: 11px; color: var(--text-tertiary); }

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
.sp-composer-spacer { flex: 1; }
.sp-attach-btn {
  display: flex; align-items: center; justify-content: center;
  width: 24px; height: 24px; border-radius: var(--radius-sm);
  color: var(--text-tertiary); flex-shrink: 0;
}
.sp-attach-btn:hover:not(:disabled) { background: var(--bg-accent); color: var(--text-secondary); }
.sp-attach-btn:disabled { opacity: 0.4; cursor: default; }
.sp-send {
  width: 28px; height: 28px; border-radius: var(--radius-md);
  background: var(--primary); color: #16202e;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0;
}
.sp-send:hover { filter: brightness(1.08); }
.sp-send:disabled { opacity: 0.4; cursor: default; filter: none; }
.sp-send.stop { background: var(--amber); }
.sp-send.stop:hover { filter: brightness(1.08); }
`;
