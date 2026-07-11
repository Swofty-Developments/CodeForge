/* Right session pane — embedded Claude Code sessions: tab strip (per session,
 * status dot + close), new-session model dropdown, a slim session header (model /
 * mode / run-state), the streamed message view, and the composer. Layout/CSS live
 * in ./session/styles; the composer + slash menu + attachments live in Composer. */

import { For, Show, createEffect, createMemo, createSignal, onCleanup } from "solid-js";
import { appStore } from "../stores/app-store";
import { MessageStream } from "./session/MessageStream";
import { SessionHeader } from "./session/SessionHeader";
import { Composer } from "./session/Composer";
import { closeSession, selectSession, startSessionWithModel } from "./session/local";
import { injectSessionStyles } from "./session/styles";
import { samePath } from "../stores/path";
import type { PastSession, PermissionMode, RunState, SessionUi } from "../types";
import * as ipc from "../ipc";

const MODEL_PRESETS: { value: string | null; name: string; desc: string }[] = [
  { value: null, name: "Default", desc: "CLI default model" },
  { value: "opus", name: "Opus", desc: "Most capable" },
  { value: "sonnet", name: "Sonnet", desc: "Fast + capable" },
  { value: "haiku", name: "Haiku", desc: "Fast + lightweight" },
  { value: "fable", name: "Fable", desc: "Extended thinking" },
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
  const [pastSessions, setPastSessions] = createSignal<PastSession[]>([]);
  const [editingSessionId, setEditingSessionId] = createSignal<string | null>(null);
  const [editingTitle, setEditingTitle] = createSignal("");

  // Load past sessions when repo changes
  createEffect(() => {
    const repo = store.repo;
    if (!repo) {
      setPastSessions([]);
      return;
    }
    void ipc.listPastSessions(repo.path).then((sessions) => {
      // Filter out sessions that are currently live
      const liveIds = new Set(store.sessions.map((s) => s.info.id));
      setPastSessions(sessions.filter((s) => !liveIds.has(s.id)));
    }).catch(console.error);
  });

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

  // Stop button = abort the in-flight turn; the chat stays. Closing the tab
  // (sp-tab-close) is the destructive stopSession path.
  function onStop(): void {
    const s = active();
    if (s) void appStore.interruptSession(s.info.id);
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

  function startEdit(sessionId: string, currentTitle: string): void {
    setEditingSessionId(sessionId);
    setEditingTitle(currentTitle);
  }

  function cancelEdit(): void {
    setEditingSessionId(null);
    setEditingTitle("");
  }

  async function finishEdit(sessionId: string): Promise<void> {
    const title = editingTitle().trim();
    if (!title) {
      cancelEdit();
      return;
    }
    await appStore.renameSession(sessionId, title);
    cancelEdit();
  }

  async function resumePastSession(claudeSessionId: string | null): Promise<void> {
    if (!claudeSessionId) return;
    await appStore.resumeSession(claudeSessionId);
    // Refresh past sessions to remove the resumed one
    const repo = store.repo;
    if (repo) {
      const sessions = await ipc.listPastSessions(repo.path);
      const liveIds = new Set(store.sessions.map((s) => s.info.id));
      setPastSessions(sessions.filter((s) => !liveIds.has(s.id)));
    }
  }

  return (
    <div
      class="session-pane"
      classList={{ "session-pane--fullscreen": store.sessionPaneFullscreen }}
      style={{ width: `${store.sessionPaneWidth}px` }}
    >
      <div class="sp-tabs">
        <div class="sp-tabs-scroll">
          <For each={visibleSessions()}>
            {(session) => (
              <div
                class="sp-tab"
                classList={{ "sp-tab--active": session.info.id === store.activeSessionId }}
                onClick={() => selectSession(session.info.id)}
                onDblClick={() => startEdit(session.info.id, session.info.title)}
                title={editingSessionId() === session.info.id ? "Editing..." : session.info.title}
              >
                <span class={`status-dot ${dotClass(session.runState)}`} />
                <Show when={editingSessionId() === session.info.id} fallback={
                  <span class="sp-tab-title">{session.info.title}</span>
                }>
                  <input
                    class="sp-tab-edit"
                    type="text"
                    value={editingTitle()}
                    onInput={(e) => setEditingTitle(e.currentTarget.value)}
                    onBlur={() => void finishEdit(session.info.id)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") void finishEdit(session.info.id);
                      if (e.key === "Escape") cancelEdit();
                    }}
                    onClick={(e) => e.stopPropagation()}
                    ref={(el) => setTimeout(() => el.select(), 0)}
                  />
                </Show>
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
          <button
            class="sp-icon-btn"
            title={store.sessionPaneFullscreen ? "Exit fullscreen" : "Fullscreen"}
            onClick={() => appStore.toggleSessionPaneFullscreen()}
          >
            <Show
              when={store.sessionPaneFullscreen}
              fallback={
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                  <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
                </svg>
              }
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                <path d="M8 3v3a2 2 0 0 1-2 2H3m18 0h-3a2 2 0 0 1-2-2V3m0 18v-3a2 2 0 0 1 2-2h3M3 16h3a2 2 0 0 1 2 2v3" />
              </svg>
            </Show>
          </button>
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
              <div class="sp-empty-title">No active session</div>
              <div class="sp-empty-sub">
                Ask for work below — a real <span class="sp-mono">claude</span> session starts
                in this repo, with hooks feeding the timeline.
              </div>
              <Show when={pastSessions().length > 0}>
                <div class="sp-past-sessions">
                  <div class="sp-past-title">Resume a session</div>
                  <div class="sp-past-list">
                    <For each={pastSessions()}>
                      {(past) => (
                        <button
                          class="sp-past-item"
                          onClick={() => void resumePastSession(past.claudeSessionId)}
                          title={`Resume "${past.title}"`}
                        >
                          <div class="sp-past-item-title">{past.title}</div>
                          <div class="sp-past-item-meta">
                            <Show when={past.model}>
                              <span class="sp-past-item-model">{past.model}</span>
                            </Show>
                            <span class="sp-past-item-date">
                              {new Date(past.updatedAt).toLocaleDateString()}
                            </span>
                          </div>
                        </button>
                      )}
                    </For>
                  </div>
                </div>
              </Show>
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
