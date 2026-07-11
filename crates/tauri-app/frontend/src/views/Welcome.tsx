/* Welcome view — open-repo CTA wired to the dialog plugin + open_repo command. */

import { Show, createSignal, onMount } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { appStore } from "../stores/app-store";
import * as ipc from "../ipc";
import type { ClaudeCliStatus } from "../types";

export function Welcome() {
  const { store } = appStore;
  const [cliStatus, setCliStatus] = createSignal<ClaudeCliStatus | null>(null);

  onMount(async () => {
    try {
      const status = await ipc.checkClaudeCli();
      setCliStatus(status);
    } catch (err) {
      console.error("Failed to check Claude CLI:", err);
    }
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
          <span>Feature</span><span class="welcome-title-accent">Forge</span>
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
        <Show when={cliStatus() && (!cliStatus()!.installed || !cliStatus()!.authenticated)}>
          <div class="welcome-cli-warning">
            <div class="welcome-cli-warning-icon">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 8v4m0 4h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z" />
              </svg>
            </div>
            <div class="welcome-cli-warning-content">
              <Show when={!cliStatus()!.installed}>
                <p class="welcome-cli-warning-title">Claude CLI not found</p>
                <p class="welcome-cli-warning-text">
                  FeatureForge requires the Claude CLI to index your codebase. Install{" "}
                  <a href="https://claude.com/claude-code" target="_blank" rel="noopener noreferrer">
                    Claude Code
                  </a>{" "}
                  and ensure <code>claude</code> is on your PATH.
                </p>
              </Show>
              <Show when={cliStatus()!.installed && !cliStatus()!.authenticated}>
                <p class="welcome-cli-warning-title">Claude CLI not authenticated</p>
                <p class="welcome-cli-warning-text">
                  Run <code>claude login</code> in your terminal to authenticate.
                </p>
              </Show>
            </div>
          </div>
        </Show>
        <Show when={store.lastError}>
          <div class="welcome-error">{store.lastError}</div>
        </Show>
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
        .welcome-cli-warning {
          display: flex;
          align-items: flex-start;
          gap: 10px;
          margin-top: var(--space-5);
          padding: 12px 14px;
          background: rgba(var(--yellow-rgb, 255, 193, 7), 0.08);
          border: 1px solid rgba(var(--yellow-rgb, 255, 193, 7), 0.25);
          border-radius: var(--radius-md);
          text-align: left;
        }
        .welcome-cli-warning-icon {
          flex-shrink: 0;
          color: rgba(var(--yellow-rgb, 255, 193, 7), 1);
          margin-top: 2px;
        }
        .welcome-cli-warning-content {
          flex: 1;
        }
        .welcome-cli-warning-title {
          font-size: 12px;
          font-weight: 600;
          color: var(--text);
          margin-bottom: 4px;
        }
        .welcome-cli-warning-text {
          font-size: 11.5px;
          line-height: 1.5;
          color: var(--text-secondary);
        }
        .welcome-cli-warning-text a {
          color: var(--primary);
          text-decoration: underline;
        }
        .welcome-cli-warning-text code {
          font-family: var(--font-mono);
          font-size: 11px;
          padding: 2px 4px;
          background: var(--bg-muted);
          border-radius: var(--radius-sm);
          color: var(--text);
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
      `}</style>
    </div>
  );
}
