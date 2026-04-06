import { createSignal, onMount, For, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";


export function SettingsOverlay() {
  const { store, setStore } = appStore;
  const [claudePath, setClaudePath] = createSignal("claude");
  const [codexPath, setCodexPath] = createSignal("codex");
  const [claudeSaveState, setClaudeSaveState] = createSignal<"idle" | "saved" | "error">("idle");
  const [codexSaveState, setCodexSaveState] = createSignal<"idle" | "saved" | "error">("idle");

  onMount(async () => {
    const cp = await ipc.getSetting("claude_path");
    const cx = await ipc.getSetting("codex_path");
    if (cp) setClaudePath(cp);
    if (cx) setCodexPath(cx);
  });

  function close() {
    setStore("settingsOpen", false);
  }

  async function savePath(key: string, value: string, setState: (s: "idle" | "saved" | "error") => void) {
    try {
      await ipc.setSetting(key, value);
      setState("saved");
      setTimeout(() => setState("idle"), 2000);
    } catch {
      setState("error");
      setTimeout(() => setState("idle"), 3000);
    }
  }

  return (
    <div class="overlay" onClick={close}>
      <div class="overlay-panel" onClick={(e) => e.stopPropagation()}>
        <div class="settings-header">
          <h3>Settings</h3>
          <button class="close-btn" onClick={close}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div class="settings-section">
          <label>Claude Binary</label>
          <span class="settings-section-desc">Override the default binary locations for AI providers</span>
          <div class="settings-input-row">
            <input
              value={claudePath()}
              onInput={(e) => setClaudePath(e.currentTarget.value)}
              onBlur={() => savePath("claude_path", claudePath(), setClaudeSaveState)}
              placeholder="claude"
            />
            <Show when={claudeSaveState() === "saved"}>
              <span class="settings-save-indicator saved">Saved</span>
            </Show>
            <Show when={claudeSaveState() === "error"}>
              <span class="settings-save-indicator error">Failed to save</span>
            </Show>
          </div>
          <button class="settings-reset-btn" onClick={() => { setClaudePath("claude"); savePath("claude_path", "claude", setClaudeSaveState); }}>
            Reset to default
          </button>
        </div>

        <div class="settings-section">
          <label>Codex Binary</label>
          <div class="settings-input-row">
            <input
              value={codexPath()}
              onInput={(e) => setCodexPath(e.currentTarget.value)}
              onBlur={() => savePath("codex_path", codexPath(), setCodexSaveState)}
              placeholder="codex"
            />
            <Show when={codexSaveState() === "saved"}>
              <span class="settings-save-indicator saved">Saved</span>
            </Show>
            <Show when={codexSaveState() === "error"}>
              <span class="settings-save-indicator error">Failed to save</span>
            </Show>
          </div>
          <button class="settings-reset-btn" onClick={() => { setCodexPath("codex"); savePath("codex_path", "codex", setCodexSaveState); }}>
            Reset to default
          </button>
        </div>


        <div class="settings-section">
          <label>Auto-name Threads</label>
          <span class="settings-section-desc">Automatically rename threads based on conversation content</span>
          <div class="settings-toggle-row">
            <span class="settings-toggle-desc">Automatically generate thread names after 3 messages</span>
            <button
              class="settings-toggle"
              classList={{ on: store.autoNamingEnabled }}
              onClick={() => setStore("autoNamingEnabled", !store.autoNamingEnabled)}
            >
              <span class="settings-toggle-knob" />
            </button>
          </div>
        </div>

        <div class="settings-section">
          <label>Desktop Notifications</label>
          <span class="settings-section-desc">Get notified when background threads complete</span>
          <div class="settings-toggle-row">
            <span class="settings-toggle-desc">Notify when background threads complete or encounter errors</span>
            <button
              class="settings-toggle"
              classList={{ on: store.notificationsEnabled }}
              onClick={() => setStore("notificationsEnabled", !store.notificationsEnabled)}
            >
              <span class="settings-toggle-knob" />
            </button>
          </div>
        </div>
      </div>

      <style>{`
        .settings-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: var(--space-5);
        }
        .settings-header h3 {
          font-size: 16px;
          font-weight: 600;
          letter-spacing: -0.3px;
        }
        .close-btn {
          color: var(--text-tertiary);
          padding: var(--space-1);
          border-radius: var(--radius-sm);
          transition: background 0.12s, color 0.12s;
          display: flex;
          align-items: center;
        }
        .close-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }
        .settings-section {
          margin-bottom: var(--space-4);
        }
        .settings-section label {
          display: block;
          font-size: 10px;
          font-weight: 600;
          color: var(--text-tertiary);
          margin-bottom: var(--space-2);
          text-transform: uppercase;
          letter-spacing: 0.06em;
        }
        .settings-section input {
          width: 100%;
          font-family: var(--font-mono);
          font-size: 12px;
          flex: 1;
        }
        .settings-input-row {
          display: flex;
          align-items: center;
          gap: var(--space-2);
        }
        .settings-save-indicator {
          font-size: 10px;
          font-weight: 600;
          white-space: nowrap;
          animation: fade-in 0.12s ease;
          flex-shrink: 0;
        }
        .settings-save-indicator.saved { color: var(--green); }
        .settings-save-indicator.error { color: var(--red); }
        .settings-reset-btn {
          font-size: 10px;
          color: var(--text-tertiary);
          margin-top: var(--space-1);
          padding: 0;
          transition: color 0.12s;
        }
        .settings-reset-btn:hover { color: var(--text-secondary); }
        .settings-section-desc {
          display: block;
          font-size: 11px;
          color: var(--text-tertiary);
          margin-bottom: var(--space-2);
          line-height: 1.4;
        }
        .settings-hint {
          display: block;
          font-size: 10px;
          color: var(--text-tertiary);
          margin-top: var(--space-1);
        }
        .settings-toggle-row {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: var(--space-3);
        }
        .settings-toggle-desc {
          font-size: 12px;
          color: var(--text-secondary);
        }
        .settings-toggle {
          width: 36px;
          height: 20px;
          border-radius: 10px;
          background: var(--bg-accent);
          border: 1px solid var(--border-strong);
          position: relative;
          transition: background 0.2s, border-color 0.2s;
          flex-shrink: 0;
          cursor: pointer;
        }
        .settings-toggle.on {
          background: var(--primary);
          border-color: var(--primary);
        }
        .settings-toggle-knob {
          position: absolute;
          top: 2px;
          left: 2px;
          width: 14px;
          height: 14px;
          border-radius: 50%;
          background: var(--text);
          transition: transform 0.2s;
        }
        .settings-toggle.on .settings-toggle-knob {
          transform: translateX(16px);
        }
      `}</style>
    </div>
  );
}
