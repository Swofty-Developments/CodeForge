import { createSignal, Show, For, onMount } from "solid-js";
import * as ipc from "../../ipc";
import type { SetupStatus, BinaryStatus } from "../../ipc";

interface Props {
  onComplete: () => void;
}

type Step = "welcome" | "binaries" | "github" | "done";

export function SetupWizard(props: Props) {
  const [step, setStep] = createSignal<Step>("welcome");
  const [status, setStatus] = createSignal<SetupStatus | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [ghLoading, setGhLoading] = createSignal(false);
  const [error, setError] = createSignal("");

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    setLoading(true);
    try {
      const s = await ipc.checkSetupStatus();
      setStatus(s);
      if (s.complete) {
        props.onComplete();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleGhLogin() {
    setGhLoading(true);
    setError("");
    try {
      await ipc.ghLogin();
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setGhLoading(false);
    }
  }

  async function handleFinish() {
    try {
      await ipc.completeSetup();
      props.onComplete();
    } catch (e) {
      setError(String(e));
    }
  }

  function canProceedFromBinaries(): boolean {
    return status()?.has_any_binary || false;
  }

  return (
    <>
    <div class="sw-overlay">
      <div class="sw-card">
        {/* Progress dots */}
        <div class="sw-progress">
          <For each={["welcome", "binaries", "github", "done"] as Step[]}>
            {(s) => (
              <div
                class="sw-dot"
                classList={{
                  "sw-dot--active": step() === s,
                  "sw-dot--done": ["welcome", "binaries", "github", "done"].indexOf(step()) > ["welcome", "binaries", "github", "done"].indexOf(s),
                }}
              />
            )}
          </For>
        </div>

        {/* Step: Welcome */}
        <Show when={step() === "welcome"}>
          <div class="sw-content">
            <div class="sw-icon-wrap">
              <svg width="44" height="44" viewBox="0 0 56 56" fill="none">
                <path d="M12 10L6 28L12 46" stroke="url(#sg1)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
                <path d="M44 10L50 28L44 46" stroke="url(#sg1)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
                <path d="M33 8L23 48" stroke="url(#sg2)" stroke-width="2.5" stroke-linecap="round" />
                <defs>
                  <linearGradient id="sg1" x1="6" y1="10" x2="50" y2="46" gradientUnits="userSpaceOnUse">
                    <stop stop-color="#6b7cff" /><stop offset="1" stop-color="#b47aff" />
                  </linearGradient>
                  <linearGradient id="sg2" x1="23" y1="48" x2="33" y2="8" gradientUnits="userSpaceOnUse">
                    <stop stop-color="#6b7cff" /><stop offset="1" stop-color="#f07ab4" />
                  </linearGradient>
                </defs>
              </svg>
            </div>
            <h1 class="sw-title">Welcome to CodeForge</h1>
            <p class="sw-subtitle">
              Your AI-powered code editor that wraps Claude Code and Codex
              into a native desktop experience.
            </p>
            <p class="sw-hint">Let's get you set up in a few quick steps.</p>
            <button class="sw-btn sw-btn--primary" onClick={() => setStep("binaries")}>
              Get Started
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg>
            </button>
          </div>
        </Show>

        {/* Step: Binary check */}
        <Show when={step() === "binaries"}>
          <div class="sw-content">
            <h2 class="sw-step-title">AI Provider Setup</h2>
            <p class="sw-step-desc">
              You need at least one AI provider installed.
            </p>

            <div class="sw-binary-list">
              <For each={status()?.binaries || []}>
                {(bin) => (
                  <div class="sw-binary" classList={{ "sw-binary--ok": bin.installed }}>
                    <div class="sw-binary-icon">
                      <Show when={bin.installed} fallback={
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--text-tertiary)" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10" /><line x1="15" y1="9" x2="9" y2="15" /><line x1="9" y1="9" x2="15" y2="15" /></svg>
                      }>
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 11-5.93-9.14" /><polyline points="22 4 12 14.01 9 11.01" /></svg>
                      </Show>
                    </div>
                    <div class="sw-binary-info">
                      <span class="sw-binary-name">
                        {bin.name === "claude" ? "Claude Code" : "Codex"}
                      </span>
                      <Show when={bin.installed}>
                        <span class="sw-binary-version">{bin.version || bin.path}</span>
                      </Show>
                      <Show when={!bin.installed}>
                        <span class="sw-binary-hint">
                          {bin.name === "claude"
                            ? "npm install -g @anthropic-ai/claude-code"
                            : "npm install -g @openai/codex"}
                        </span>
                      </Show>
                    </div>
                  </div>
                )}
              </For>
            </div>

            <Show when={!canProceedFromBinaries()}>
              <p class="sw-warning">Install at least one provider to continue.</p>
            </Show>

            <div class="sw-actions">
              <button class="sw-btn sw-btn--ghost" onClick={() => setStep("welcome")}>Back</button>
              <button class="sw-btn sw-btn--ghost" onClick={refresh}>
                {loading() ? "Checking…" : "Re-check"}
              </button>
              <button
                class="sw-btn sw-btn--primary"
                onClick={() => setStep("github")}
                disabled={!canProceedFromBinaries()}
              >
                Continue
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><polyline points="9 18 15 12 9 6" /></svg>
              </button>
            </div>
          </div>
        </Show>

        {/* Step: GitHub auth */}
        <Show when={step() === "github"}>
          <div class="sw-content">
            <h2 class="sw-step-title">Connect GitHub</h2>
            <p class="sw-step-desc">
              Link your GitHub account for PR dashboards, issue context,
              and repository-aware threads.
            </p>

            <Show when={!status()?.gh_installed}>
              <div class="sw-gh-status sw-gh-status--warn">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--amber)" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
                <div>
                  <strong>GitHub CLI not found</strong>
                  <p>Install it: <code>brew install gh</code></p>
                </div>
              </div>
            </Show>

            <Show when={status()?.gh_installed && !status()?.gh_authenticated}>
              <div class="sw-gh-status">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="var(--text-secondary)">
                  <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
                </svg>
                <div>
                  <strong>Not authenticated</strong>
                  <p>Connect via GitHub CLI to enable full integration.</p>
                </div>
              </div>
              <button
                class="sw-btn sw-btn--github"
                onClick={handleGhLogin}
                disabled={ghLoading()}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
                </svg>
                {ghLoading() ? "Authenticating…" : "Sign in with GitHub"}
              </button>
            </Show>

            <Show when={status()?.gh_authenticated}>
              <div class="sw-gh-status sw-gh-status--ok">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 11-5.93-9.14" /><polyline points="22 4 12 14.01 9 11.01" /></svg>
                <div>
                  <strong>Connected as {status()!.gh_username || "user"}</strong>
                  <p>GitHub integration is ready.</p>
                </div>
              </div>
            </Show>

            <Show when={error()}>
              <p class="sw-error">{error()}</p>
            </Show>

            <div class="sw-actions">
              <button class="sw-btn sw-btn--ghost" onClick={() => setStep("binaries")}>Back</button>
              <button class="sw-btn sw-btn--primary" onClick={() => setStep("done")}>
                {status()?.gh_authenticated ? "Continue" : "Skip for now"}
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><polyline points="9 18 15 12 9 6" /></svg>
              </button>
            </div>
          </div>
        </Show>

        {/* Step: Done */}
        <Show when={step() === "done"}>
          <div class="sw-content">
            <div class="sw-done-icon">
              <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 11-5.93-9.14" /><polyline points="22 4 12 14.01 9 11.01" /></svg>
            </div>
            <h2 class="sw-step-title">You're all set</h2>
            <p class="sw-step-desc">
              CodeForge is ready. Create a thread and start coding.
            </p>
            <div class="sw-summary">
              <For each={status()?.binaries.filter((b) => b.installed) || []}>
                {(bin) => (
                  <div class="sw-summary-item">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2.5" stroke-linecap="round"><polyline points="20 6 9 17 4 12" /></svg>
                    {bin.name === "claude" ? "Claude Code" : "Codex"}
                  </div>
                )}
              </For>
              <Show when={status()?.gh_authenticated}>
                <div class="sw-summary-item">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2.5" stroke-linecap="round"><polyline points="20 6 9 17 4 12" /></svg>
                  GitHub ({status()!.gh_username})
                </div>
              </Show>
            </div>
            <button class="sw-btn sw-btn--primary sw-btn--lg" onClick={handleFinish}>
              Launch CodeForge
            </button>
          </div>
        </Show>
      </div>
    </div>
    <style>{`
    .sw-overlay {
      position: fixed;
      inset: 0;
      background: var(--bg-base);
      background-image:
        radial-gradient(ellipse 60% 50% at 50% 40%, rgba(107, 124, 255, 0.06) 0%, transparent 70%),
        radial-gradient(ellipse 40% 30% at 30% 70%, rgba(180, 122, 255, 0.04) 0%, transparent 60%),
        radial-gradient(ellipse 35% 25% at 70% 20%, rgba(240, 122, 180, 0.03) 0%, transparent 60%);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 200;
      animation: sw-bg-in 0.8s ease both;
    }
    @keyframes sw-bg-in {
      from { opacity: 0; }
      to { opacity: 1; }
    }
    .sw-card {
      width: 460px;
      max-width: 90vw;
      display: flex;
      flex-direction: column;
      align-items: center;
    }

    /* Progress */
    .sw-progress {
      display: flex;
      gap: 8px;
      margin-bottom: 32px;
    }
    .sw-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--bg-accent);
      transition: all 0.2s;
    }
    .sw-dot--active {
      background: var(--primary);
      box-shadow: 0 0 8px var(--primary-glow);
      transform: scale(1.2);
    }
    .sw-dot--done {
      background: var(--green);
    }

    /* Content — fades in on each step */
    @keyframes sw-step-in {
      from { opacity: 0; transform: translateY(8px); }
      to { opacity: 1; transform: translateY(0); }
    }
    .sw-content {
      display: flex;
      flex-direction: column;
      align-items: center;
      text-align: center;
      width: 100%;
      animation: sw-step-in 0.3s cubic-bezier(0.16, 1, 0.3, 1) both;
    }
    .sw-icon-wrap {
      margin-bottom: 20px;
      opacity: 0.9;
    }
    .sw-title {
      font-size: 26px;
      font-weight: 700;
      letter-spacing: -0.8px;
      color: var(--text);
      margin-bottom: 10px;
    }
    .sw-subtitle {
      font-size: 14px;
      color: var(--text-secondary);
      line-height: 1.6;
      max-width: 380px;
      margin-bottom: 6px;
    }
    .sw-hint {
      font-size: 12px;
      color: var(--text-tertiary);
      margin-bottom: 24px;
    }
    .sw-step-title {
      font-size: 20px;
      font-weight: 700;
      color: var(--text);
      letter-spacing: -0.4px;
      margin-bottom: 8px;
    }
    .sw-step-desc {
      font-size: 13px;
      color: var(--text-secondary);
      line-height: 1.5;
      max-width: 360px;
      margin-bottom: 20px;
    }

    /* Buttons */
    .sw-btn {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 10px 20px;
      border-radius: var(--radius-md);
      font-size: 13px;
      font-weight: 600;
      transition: all 0.15s;
      cursor: pointer;
    }
    .sw-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }
    .sw-btn--primary {
      background: var(--primary);
      color: white;
    }
    .sw-btn--primary:hover:not(:disabled) { filter: brightness(1.1); transform: translateY(-1px); }
    .sw-btn--lg { padding: 12px 28px; font-size: 14px; }
    .sw-btn--ghost {
      color: var(--text-tertiary);
      padding: 8px 14px;
    }
    .sw-btn--ghost:hover { color: var(--text-secondary); background: var(--bg-hover); }
    .sw-btn--github {
      background: linear-gradient(135deg, #24292e 0%, #1a1e22 100%);
      color: white;
      padding: 10px 20px;
      border-radius: var(--radius-md);
      margin-top: 12px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
      transition: all 0.15s;
    }
    .sw-btn--github:hover:not(:disabled) {
      background: linear-gradient(135deg, #2f363d 0%, #24292e 100%);
      border-color: rgba(255, 255, 255, 0.12);
      transform: translateY(-1px);
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    }
    .sw-actions {
      display: flex;
      gap: 8px;
      margin-top: 24px;
      width: 100%;
      justify-content: center;
    }

    /* Binary list */
    .sw-binary-list {
      width: 100%;
      display: flex;
      flex-direction: column;
      gap: 8px;
      margin-bottom: 16px;
    }
    .sw-binary {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 12px 16px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      text-align: left;
    }
    .sw-binary--ok {
      border-color: rgba(76, 214, 148, 0.2);
    }
    .sw-binary-icon { flex-shrink: 0; }
    .sw-binary-info { display: flex; flex-direction: column; gap: 2px; }
    .sw-binary-name {
      font-size: 14px;
      font-weight: 600;
      color: var(--text);
    }
    .sw-binary-version {
      font-size: 11px;
      color: var(--text-tertiary);
      font-family: var(--font-mono);
    }
    .sw-binary-hint {
      font-size: 11px;
      color: var(--text-tertiary);
      font-family: var(--font-mono);
    }
    .sw-warning {
      font-size: 12px;
      color: var(--amber);
      margin-top: 4px;
    }
    .sw-error {
      font-size: 12px;
      color: var(--red);
      margin-top: 8px;
    }

    /* GitHub status */
    .sw-gh-status {
      display: flex;
      align-items: flex-start;
      gap: 12px;
      padding: 14px 16px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      text-align: left;
      width: 100%;
    }
    .sw-gh-status strong {
      font-size: 13px;
      color: var(--text);
      display: block;
      margin-bottom: 2px;
    }
    .sw-gh-status p {
      font-size: 12px;
      color: var(--text-tertiary);
      margin: 0;
    }
    .sw-gh-status code {
      font-family: var(--font-mono);
      font-size: 11px;
      background: var(--bg-accent);
      padding: 1px 5px;
      border-radius: 3px;
    }
    .sw-gh-status--ok { border-color: rgba(76, 214, 148, 0.2); }
    .sw-gh-status--warn { border-color: rgba(240, 184, 64, 0.2); }
    .sw-gh-status svg { flex-shrink: 0; margin-top: 1px; }

    /* Done summary */
    .sw-done-icon { margin-bottom: 16px; }
    .sw-summary {
      display: flex;
      flex-direction: column;
      gap: 6px;
      margin-bottom: 24px;
    }
    .sw-summary-item {
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 13px;
      color: var(--text-secondary);
    }
    `}</style>
    </>
  );
}
