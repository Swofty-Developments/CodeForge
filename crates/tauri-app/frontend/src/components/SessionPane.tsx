/* Right session pane — embedded Claude Code sessions: tab strip (per session,
 * status dot + close), new-session model dropdown, the streamed message view,
 * and the autosize composer. Layout/CSS live in ./session/styles. */

import { For, Show, createEffect, createSignal } from "solid-js";
import { appStore } from "../stores/app-store";
import { MessageStream } from "./session/MessageStream";
import { closeSession, selectSession, startSessionWithModel } from "./session/local";
import { injectSessionStyles } from "./session/styles";
import type { RunState, SessionUi } from "../types";

const MAX_COMPOSER_PX = 168; // ~8 lines at 13.5px / 1.45

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

function autosize(el: HTMLTextAreaElement): void {
  el.style.height = "auto";
  el.style.height = `${Math.min(el.scrollHeight, MAX_COMPOSER_PX)}px`;
}

export function SessionPane() {
  injectSessionStyles();
  const { store } = appStore;
  const [input, setInput] = createSignal("");
  const [menuOpen, setMenuOpen] = createSignal(false);
  const [custom, setCustom] = createSignal("");
  let taRef: HTMLTextAreaElement | undefined;

  const active = (): SessionUi | null =>
    store.sessions.find((s) => s.info.id === store.activeSessionId) ?? null;
  const generating = () => active()?.runState === "generating";
  const canSend = () => !!store.repo && !!input().trim();

  // "Ask Claude about this feature" seeds composerPrefill + reveals the pane.
  createEffect(() => {
    const pf = store.composerPrefill;
    if (pf == null) return;
    setInput(pf);
    appStore.clearComposerPrefill();
    queueMicrotask(() => {
      if (taRef) { taRef.focus(); autosize(taRef); }
    });
  });

  async function submit(): Promise<void> {
    const text = input().trim();
    if (!text || !store.repo) return;
    setInput("");
    if (taRef) autosize(taRef);
    if (!store.activeSessionId) await appStore.startSession();
    // sendSessionInput pushes the user message into session.messages itself.
    await appStore.sendSessionInput(text);
  }

  function pickModel(model: string | null): void {
    setMenuOpen(false);
    setCustom("");
    void startSessionWithModel(model ?? undefined);
  }

  function pickCustom(): void {
    const m = custom().trim();
    if (m) pickModel(m);
  }

  return (
    <div class="session-pane" style={{ width: `${store.sessionPaneWidth}px` }}>
      <div class="sp-tabs">
        <div class="sp-tabs-scroll">
          <For each={store.sessions}>
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

      <div class="sp-composer-wrap">
        <div class="sp-composer-card" classList={{ generating: generating(), disabled: !store.repo }}>
          <textarea
            ref={taRef}
            class="sp-input"
            rows={1}
            placeholder={store.repo ? "Ask Claude about this repo…" : "Open a repository first"}
            disabled={!store.repo}
            value={input()}
            onInput={(e) => { setInput(e.currentTarget.value); autosize(e.currentTarget); }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); void submit(); }
            }}
          />
          <div class="sp-composer-meta">
            <Show when={active()}>
              {(s) => <span class="sp-model-tag">{s().info.model ?? "default"}</span>}
            </Show>
            <div class="sp-composer-spacer" />
            <Show
              when={generating()}
              fallback={
                <button class="sp-send" title="Send" disabled={!canSend()} onClick={() => void submit()}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                    <path d="M12 19V5M5 12l7-7 7 7" />
                  </svg>
                </button>
              }
            >
              <button
                class="sp-send stop"
                title="Stop session"
                onClick={() => { const s = active(); if (s) void appStore.stopSession(s.info.id); }}
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="6" y="6" width="12" height="12" rx="2" />
                </svg>
              </button>
            </Show>
          </div>
        </div>
      </div>
    </div>
  );
}
