/* Right session pane — embedded Claude Code sessions: tab strip (per session,
 * status dot + close), new-session model dropdown, a slim session header (model /
 * mode / run-state), the streamed message view, and the composer. Layout/CSS live
 * in ./session/styles; the composer + slash menu + attachments live in Composer. */

import { For, Show, createMemo, createSignal } from "solid-js";
import { appStore } from "../stores/app-store";
import { MessageStream } from "./session/MessageStream";
import { SessionHeader } from "./session/SessionHeader";
import { Composer } from "./session/Composer";
import { closeSession, selectSession, startSessionWithModel } from "./session/local";
import { injectSessionStyles } from "./session/styles";
import { samePath } from "../stores/path";
import type { PermissionMode, RunState, SessionUi } from "../types";

const MODEL_PRESETS: { value: string | null; name: string; desc: string }[] = [
  { value: null, name: "Default", desc: "CLI default model" },
  { value: "opus", name: "Opus", desc: "Most capable" },
  { value: "sonnet", name: "Sonnet", desc: "Fast + capable" },
  { value: "haiku", name: "Haiku", desc: "Fast + lightweight" },
];

function dotClass(runState: RunState): string {
  switch (runState) {
    case "ready": return "status-dot--ready";
    case "generating": return "status-dot--busy";
    case "starting": return "status-dot--waiting";
    case "error": return "status-dot--error";
    default: return "";
  }
}

export function SessionPane() {
  injectSessionStyles();
  const { store } = appStore;
  const [menuOpen, setMenuOpen] = createSignal(false);
  const [custom, setCustom] = createSignal("");
  // Mode for the NEXT session started here; an active session shows its own live mode.
  const [pendingMode, setPendingMode] = createSignal<PermissionMode>("default");

  // Only the active context's sessions are shown (W1: sessions are context-tagged).
  const visibleSessions = createMemo((): SessionUi[] =>
    store.activeContextPath
      ? store.sessions.filter((s) => samePath(s.contextPath, store.activeContextPath!))
      : [],
  );
  const active = (): SessionUi | null => {
    const id = store.activeSessionId;
    if (!id) return null;
    return visibleSessions().find((s) => s.info.id === id) ?? null;
  };
  const currentMode = (): PermissionMode => active()?.permissionMode ?? pendingMode();

  function selectMode(mode: PermissionMode): void {
    setPendingMode(mode);
    const s = active();
    if (s) void appStore.setSessionMode(s.info.id, mode);
  }

  async function onSend(text: string): Promise<void> {
    if (!store.repo) return;
    if (!active()) await appStore.startSession(undefined, pendingMode());
    // sendSessionInput pushes the user message into session.messages itself.
    await appStore.sendSessionInput(text);
  }

  function onStop(): void {
    const s = active();
    if (s) void appStore.stopSession(s.info.id);
  }

  function pickModel(model: string | null): void {
    setMenuOpen(false);
    setCustom("");
    void startSessionWithModel(model ?? undefined, pendingMode());
  }

  function pickCustom(): void {
    const m = custom().trim();
    if (m) pickModel(m);
  }

  return (
    <div class="session-pane" style={{ width: `${store.sessionPaneWidth}px` }}>
      <div class="sp-tabs">
        <div class="sp-tabs-scroll">
          <For each={visibleSessions()}>
            {(session) => (
              <div
                class="sp-tab"
                classList={{ "sp-tab--active": session.info.id === store.activeSessionId }}
                onClick={() => selectSession(session.info.id)}
                title={session.info.title}
              >
                <span class={`status-dot ${dotClass(session.runState)}`} />
                <span class="sp-tab-title">{session.info.title}</span>
                <button
                  class="sp-tab-close"
                  title="Stop session"
                  onClick={(e) => { e.stopPropagation(); void closeSession(session.info.id); }}
                >
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <path d="M18 6L6 18M6 6l12 12" />
                  </svg>
                </button>
              </div>
            )}
          </For>
        </div>

        <div class="sp-tab-actions">
          <div class="sp-new-wrap">
            <button
              class="sp-icon-btn"
              title="New session"
              disabled={!store.repo}
              onClick={() => setMenuOpen((o) => !o)}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                <path d="M12 5v14M5 12h14" />
              </svg>
            </button>
            <Show when={menuOpen()}>
              <div class="sp-menu-backdrop" onClick={() => setMenuOpen(false)} />
              <div class="sp-model-menu">
                <div class="sp-menu-label">New session</div>
                <For each={MODEL_PRESETS}>
                  {(p) => (
                    <button class="sp-menu-item" onClick={() => pickModel(p.value)}>
                      <span class="sp-menu-item-name">{p.name}</span>
                      <span class="sp-menu-item-desc">{p.desc}</span>
                    </button>
                  )}
                </For>
                <div class="sp-menu-custom">
                  <input
                    class="sp-menu-input"
                    placeholder="custom model…"
                    value={custom()}
                    onInput={(e) => setCustom(e.currentTarget.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") { e.preventDefault(); pickCustom(); }
                    }}
                  />
                </div>
              </div>
            </Show>
          </div>
          <button class="sp-icon-btn" title="Collapse pane" onClick={() => appStore.toggleSessionPane()}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
              <path d="M13 18l6-6-6-6M6 18l6-6-6-6" />
            </svg>
          </button>
        </div>
      </div>

      <Show when={active()}>
        {(session) => <SessionHeader session={session()} />}
      </Show>

      <div class="sp-body">
        <Show
          when={active()}
          fallback={
            <div class="sp-empty">
              <div class="sp-empty-title">No session yet</div>
              <div class="sp-empty-sub">
                Ask for work below — a real <span class="sp-mono">claude</span> session starts
                in this repo, with hooks feeding the timeline.
              </div>
            </div>
          }
        >
          {(session) => <MessageStream session={session()} />}
        </Show>
      </div>

      <Composer
        disabled={!store.repo}
        session={active()}
        currentMode={currentMode()}
        onSelectMode={selectMode}
        onSend={onSend}
        onStop={onStop}
      />
    </div>
  );
}
