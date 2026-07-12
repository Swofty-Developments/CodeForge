/* Welcome view — open-repo CTA wired to the dialog plugin + open_repo command,
 * plus the Claude CLI health check (ok / broken / notFound, named states). */

import { Show, Switch, Match, createSignal, onMount } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { appStore } from "../stores/app-store";
import { claudeCliStatus } from "../ipc";
import type { ClaudeAuthStatus, ClaudeCliStatus } from "../types";

export function Welcome() {
  const { store } = appStore;
  const [cli, setCli] = createSignal<ClaudeCliStatus | null>(null);

  onMount(() => {
    claudeCliStatus()
      .then(setCli)
      .catch((e) => appStore.pushError(`Claude CLI check failed: ${String(e)}`));
  });

  async function pickRepo() {
    const dir = await open({ directory: true, multiple: false, title: "Open a repository" });
    if (typeof dir === "string") await appStore.openRepo(dir);
  }

  return (
    <div class="welcome">
      <div class="welcome-inner">
        <div class="welcome-mark">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="url(#ffgrad)" stroke-width="1.6">
            <defs>
              <linearGradient id="ffgrad" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" stop-color="var(--primary)" />
                <stop offset="100%" stop-color="var(--purple)" />
              </linearGradient>
            </defs>
            <path d="M4 17h9l3 3H7l-3-3zM6 12h12M8 7h12l-2 3H8l2-3z" />
          </svg>
        </div>
        <h1 class="welcome-title">
          <span>Code</span><span class="welcome-title-accent">Forge</span>
        </h1>
        <p class="welcome-sub">
          Open a repository and see <em>features</em>, not files.
        </p>
        <button class="welcome-cta" onClick={() => void pickRepo()}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
          </svg>
          Open repository
        </button>
        <div class="welcome-hints">
          <span class="kbd-hint"><kbd>⌘K</kbd> command palette</span>
          <span class="kbd-hint"><kbd>⌘\</kbd> session pane</span>
        </div>
        <Show when={store.lastError}>
          <div class="welcome-error">{store.lastError}</div>
        </Show>

        <Switch>
          <Match when={cli()?.state === "ok" && cli()}>
            {(ok) => {
              const s = ok() as Extract<ClaudeCliStatus, { state: "ok" }>;
              return (
                <>
                  <div class="welcome-cli-ok">
                    {s.version} · {s.path}
                    <Show when={s.auth.state === "loggedIn" && s.auth}>
                      {(auth) => {
                        const a = auth() as Extract<ClaudeAuthStatus, { state: "loggedIn" }>;
                        return <> · {a.email ?? a.method ?? "logged in"}{a.subscription ? ` (${a.subscription})` : ""}</>;
                      }}
                    </Show>
                  </div>
                  <Show when={s.auth.state === "loggedOut"}>
                    <div class="welcome-cli-warn">
                      <div class="welcome-cli-warn-title">Claude Code is not logged in</div>
                      <div>Sessions and indexing will fail until you sign in:</div>
                      <code>claude auth login</code>
                    </div>
                  </Show>
                  <Show when={s.auth.state === "unknown" && s.auth}>
                    {(auth) => {
                      const a = auth() as Extract<ClaudeAuthStatus, { state: "unknown" }>;
                      return <div class="welcome-cli-ok">auth status unavailable: {a.detail}</div>;
                    }}
                  </Show>
                </>
              );
            }}
          </Match>
          <Match when={cli()?.state === "notFound" && cli()}>
            {(missing) => (
              <div class="welcome-cli-warn">
                <div class="welcome-cli-warn-title">Claude Code CLI not found</div>
                <div>Sessions and indexing need it. Install with either:</div>
                <code>brew install --cask claude-code</code>
                <code>curl -fsSL https://claude.ai/install.sh | bash</code>
                <Show when={!missing().shellEnvResolved}>
                  <div class="welcome-cli-warn-note">
                    Your login-shell PATH could not be read, so an existing install outside the
                    standard locations may also be invisible to CodeForge.
                  </div>
                </Show>
              </div>
            )}
          </Match>
          <Match when={cli()?.state === "broken" && cli()}>
            {(broken) => {
              const s = broken() as Extract<ClaudeCliStatus, { state: "broken" }>;
              return (
                <div class="welcome-cli-warn">
                  <div class="welcome-cli-warn-title">Claude Code CLI found but not runnable</div>
                  <code>{s.path}</code>
                  <div class="welcome-cli-warn-note">{s.detail}</div>
                </div>
              );
            }}
          </Match>
        </Switch>
      </div>

      <style>{`
        .welcome {
          flex: 1;
          display: flex;
          align-items: center;
          justify-content: center;
          background: var(--bg-base);
          position: relative;
          overflow: hidden;
        }
        .welcome::before {
          content: "";
          position: absolute;
          top: 50%; left: 50%;
          width: 600px; height: 600px;
          transform: translate(-50%, -50%);
          background: radial-gradient(circle, var(--primary-glow) 0%, transparent 65%);
          opacity: 0.35;
          pointer-events: none;
        }
        .welcome-inner {
          position: relative;
          text-align: center;
          animation: ws-content-in 0.4s var(--ease-out) both;
          max-width: 420px;
          padding: var(--space-6);
        }
        .welcome-mark { margin-bottom: var(--space-4); animation: ws-mark-in 0.5s var(--ease-out) both; }
        .welcome-title {
          font-size: 34px; font-weight: 700; letter-spacing: -1.2px;
          color: var(--text);
          animation: ws-title-in 0.5s var(--ease-out) 0.1s both;
        }
        .welcome-title-accent { color: var(--primary); }
        .welcome-sub {
          margin-top: var(--space-2);
          font-size: 13px;
          color: var(--text-tertiary);
        }
        .welcome-sub em { color: var(--text-secondary); font-style: normal; font-weight: 600; }
        .welcome-cta {
          display: inline-flex; align-items: center; justify-content: center; gap: 8px;
          margin-top: var(--space-6);
          padding: 10px 28px;
          font-size: 14px; font-weight: 600;
          color: #16202e;
          background: var(--primary);
          border-radius: var(--radius-md);
        }
        .welcome-cta:hover { filter: brightness(1.08); }
        .welcome-hints {
          display: flex; justify-content: center; gap: var(--space-4);
          margin-top: var(--space-6);
        }
        .kbd-hint { font-size: 11px; color: var(--text-tertiary); }
        .kbd-hint kbd {
          padding: 2px 6px;
          background: var(--bg-muted);
          border: 1px solid var(--border);
          border-radius: var(--radius-sm);
          font-size: 10px;
          font-family: var(--font-mono);
          margin-right: 4px;
        }
        .welcome-error {
          margin-top: var(--space-4);
          font-size: 11px; font-family: var(--font-mono);
          padding: 8px 12px; border-radius: var(--radius-sm);
          background: rgba(var(--red-rgb), 0.08);
          border: 1px solid rgba(var(--red-rgb), 0.2);
          color: var(--red);
          word-break: break-word;
          user-select: text; -webkit-user-select: text;
        }
        .welcome-cli-ok {
          margin-top: var(--space-5);
          font-size: 10px; font-family: var(--font-mono);
          color: var(--text-tertiary);
        }
        .welcome-cli-warn {
          margin-top: var(--space-5);
          text-align: left;
          display: flex; flex-direction: column; gap: 6px;
          font-size: 11px;
          padding: 10px 12px; border-radius: var(--radius-sm);
          background: rgba(var(--amber-rgb), 0.08);
          border: 1px solid rgba(var(--amber-rgb), 0.2);
          color: var(--text-secondary);
        }
        .welcome-cli-warn-title { color: var(--amber); font-weight: 600; }
        .welcome-cli-warn code {
          font-family: var(--font-mono); font-size: 10.5px;
          padding: 3px 6px; border-radius: var(--radius-sm);
          background: var(--bg-muted); border: 1px solid var(--border);
          color: var(--text);
          user-select: text; -webkit-user-select: text;
          width: fit-content;
        }
        .welcome-cli-warn-note { color: var(--text-tertiary); word-break: break-word; }
      `}</style>
    </div>
  );
}
