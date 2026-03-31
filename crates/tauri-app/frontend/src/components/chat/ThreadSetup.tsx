import { createSignal, For, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import { PrDashboard } from "../github/PrDashboard";
import type { PullRequest } from "../../ipc";

type Mode = "pick" | "branch" | "pr";

interface Props {
  projectId: string;
  repoPath: string;
  threadId: string;
}

export function ThreadSetup(props: Props) {
  const { store, setStore } = appStore;
  const [mode, setMode] = createSignal<Mode>("pick");
  const [branches, setBranches] = createSignal<{ name: string; current: boolean }[]>([]);
  const [loadingBranches, setLoadingBranches] = createSignal(false);
  const [creating, setCreating] = createSignal(false);

  async function loadBranches() {
    setLoadingBranches(true);
    try {
      const list = await ipc.gitBranches(props.repoPath);
      setBranches(list);
    } catch (e) {
      console.error("Failed to load branches:", e);
    } finally {
      setLoadingBranches(false);
    }
  }

  async function startOnBranch(branchName: string, createWorktree: boolean) {
    if (creating()) return;
    setCreating(true);
    try {
      if (createWorktree) {
        const thread = store.projects.flatMap((p) => p.threads).find((t) => t.id === props.threadId);
        const wt = await ipc.createWorktree(props.threadId, thread?.title || "worktree", props.repoPath);
        setStore("worktrees", props.threadId, wt);
        // Checkout the requested branch in the worktree
        if (branchName !== "main" && branchName !== "master") {
          await ipc.gitCheckout(wt.path, branchName);
        }
      }
      // Rename thread to include branch
      const newTitle = createWorktree ? `[${branchName}] ${store.projects.flatMap((p) => p.threads).find((t) => t.id === props.threadId)?.title || "Thread"}` : undefined;
      if (newTitle) {
        await ipc.renameThread(props.threadId, newTitle);
        setStore("projects", (projects) =>
          projects.map((p) => ({
            ...p,
            threads: p.threads.map((t) => t.id === props.threadId ? { ...t, title: newTitle } : t),
          }))
        );
      }
      // Add a context message — include worktree path
      const wt = store.worktrees[props.threadId];
      const context = createWorktree && wt
        ? `Working on branch \`${branchName}\` in a worktree at ${wt.path}. What would you like to do?`
        : `Working on the main branch at ${props.repoPath}. What would you like to do?`;
      const msgId = await ipc.persistUserMessage(props.threadId, context);
      setStore("threadMessages", props.threadId, (msgs) => [
        ...(msgs || []),
        { id: msgId, thread_id: props.threadId, role: "user" as const, content: context },
      ]);
    } catch (e) {
      console.error("Failed to set up thread:", e);
    } finally {
      setCreating(false);
    }
  }

  async function linkPr(pr: PullRequest) {
    if (creating()) return;
    setCreating(true);
    try {
      // Rename thread
      const title = `PR #${pr.number}: ${pr.title}`;
      await ipc.renameThread(props.threadId, title);
      setStore("projects", (projects) =>
        projects.map((p) => ({
          ...p,
          threads: p.threads.map((t) => t.id === props.threadId ? { ...t, title } : t),
        }))
      );

      // Store PR link
      ipc.setSetting(`pr:${props.threadId}`, String(pr.number)).catch(() => {});
      setStore("projectPrMap", props.projectId, (map) => ({
        ...(map || {}),
        [props.threadId]: pr.number,
      }));

      // Create worktree on the PR branch
      try {
        const wt = await ipc.createWorktree(props.threadId, title, props.repoPath);
        setStore("worktrees", props.threadId, wt);
        // Checkout the PR branch
        await ipc.gitCheckout(wt.path, pr.branch);
      } catch (e) {
        console.error("Worktree creation failed (continuing without):", e);
      }

      // Inject context — include worktree path so the agent knows where to work
      const wt = store.worktrees[props.threadId];
      const wtInfo = wt ? `\n\nWorktree: ${wt.path} (branch: ${wt.branch})` : "";
      const context = `I'm working on PR #${pr.number}: "${pr.title}" by ${pr.author}\n\nBranch: ${pr.branch} → ${pr.base}\n+${pr.additions} -${pr.deletions} across ${pr.changed_files} files\n${pr.labels.length > 0 ? `Labels: ${pr.labels.join(", ")}\n` : ""}${wtInfo}\n\nHelp me review or continue work on this PR.`;
      const msgId = await ipc.persistUserMessage(props.threadId, context);
      setStore("threadMessages", props.threadId, (msgs) => [
        ...(msgs || []),
        { id: msgId, thread_id: props.threadId, role: "user" as const, content: context },
      ]);
    } catch (e) {
      console.error("Failed to link PR:", e);
    } finally {
      setCreating(false);
    }
  }

  return (
    <div class="ts">
      <Show when={mode() === "pick"}>
        <div class="ts-header">
          <h3 class="ts-title">How do you want to start?</h3>
          <p class="ts-subtitle">Choose a starting point for this thread</p>
        </div>
        <div class="ts-pods">
          <button class="ts-pod" onClick={() => startOnBranch("main", false)}>
            <div class="ts-pod-icon">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
              </svg>
            </div>
            <span class="ts-pod-title">Empty</span>
            <span class="ts-pod-desc">Start on the main branch</span>
          </button>

          <button class="ts-pod" onClick={() => { setMode("branch"); loadBranches(); }}>
            <div class="ts-pod-icon">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M6 21V9a9 9 0 009 9" />
              </svg>
            </div>
            <span class="ts-pod-title">With Worktree</span>
            <span class="ts-pod-desc">Pick a branch, isolated workspace</span>
          </button>

          <button class="ts-pod" onClick={() => setMode("pr")}>
            <div class="ts-pod-icon">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M13 6h3a2 2 0 012 2v7" /><line x1="6" y1="9" x2="6" y2="21" />
              </svg>
            </div>
            <span class="ts-pod-title">From Pull Request</span>
            <span class="ts-pod-desc">Link a PR with auto worktree</span>
          </button>
        </div>
      </Show>

      <Show when={mode() === "branch"}>
        <div class="ts-header">
          <button class="ts-back" onClick={() => setMode("pick")}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6" /></svg>
            Back
          </button>
          <h3 class="ts-title">Select a branch</h3>
        </div>
        <Show when={loadingBranches()}>
          <div class="ts-loading">Loading branches…</div>
        </Show>
        <div class="ts-branch-list">
          <For each={branches()}>
            {(branch) => (
              <button
                class="ts-branch"
                classList={{ "ts-branch--current": branch.current }}
                onClick={() => startOnBranch(branch.name, true)}
                disabled={creating()}
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M6 21V9a9 9 0 009 9" />
                </svg>
                <span class="ts-branch-name">{branch.name}</span>
                {branch.current && <span class="ts-branch-badge">current</span>}
              </button>
            )}
          </For>
        </div>
      </Show>

      <Show when={mode() === "pr"}>
        <div class="ts-header">
          <button class="ts-back" onClick={() => setMode("pick")}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6" /></svg>
            Back
          </button>
          <h3 class="ts-title">Select a Pull Request</h3>
        </div>
        <PrDashboard projectId={props.projectId} repoPath={props.repoPath} onLinkPr={linkPr} />
      </Show>
    </div>
  );
}

if (!document.getElementById("thread-setup-styles")) {
  const s = document.createElement("style");
  s.id = "thread-setup-styles";
  s.textContent = `
    .ts {
      max-width: 580px;
      margin: 0 auto;
      padding: 24px 16px;
      width: 100%;
      animation: fade-slide-up 0.2s ease both;
    }
    .ts-header {
      margin-bottom: 20px;
    }
    .ts-title {
      font-size: 18px;
      font-weight: 700;
      color: var(--text);
      letter-spacing: -0.3px;
    }
    .ts-subtitle {
      font-size: 13px;
      color: var(--text-tertiary);
      margin-top: 4px;
    }
    .ts-back {
      display: inline-flex;
      align-items: center;
      gap: 4px;
      font-size: 12px;
      color: var(--text-tertiary);
      margin-bottom: 8px;
      padding: 3px 6px;
      border-radius: var(--radius-sm);
      transition: all 0.1s;
    }
    .ts-back:hover { color: var(--text-secondary); background: var(--bg-hover); }

    /* Pods */
    .ts-pods {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 10px;
    }
    .ts-pod {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 8px;
      padding: 20px 12px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      text-align: center;
      cursor: pointer;
      transition: all 0.15s;
    }
    .ts-pod:hover {
      border-color: var(--border-strong);
      background: var(--bg-accent);
      transform: translateY(-2px);
      box-shadow: 0 4px 16px rgba(0,0,0,0.2);
    }
    .ts-pod-icon {
      color: var(--primary);
      opacity: 0.8;
    }
    .ts-pod:hover .ts-pod-icon { opacity: 1; }
    .ts-pod-title {
      font-size: 13px;
      font-weight: 600;
      color: var(--text);
    }
    .ts-pod-desc {
      font-size: 11px;
      color: var(--text-tertiary);
      line-height: 1.3;
    }

    /* Branch list */
    .ts-branch-list {
      display: flex;
      flex-direction: column;
      gap: 3px;
    }
    .ts-branch {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px 12px;
      border-radius: var(--radius-sm);
      font-size: 13px;
      color: var(--text-secondary);
      transition: all 0.1s;
      text-align: left;
    }
    .ts-branch:hover { background: var(--bg-accent); color: var(--text); }
    .ts-branch:disabled { opacity: 0.5; }
    .ts-branch--current { color: var(--primary); }
    .ts-branch svg { color: var(--text-tertiary); flex-shrink: 0; }
    .ts-branch--current svg { color: var(--primary); }
    .ts-branch-name {
      font-family: var(--font-mono);
      font-size: 12px;
      flex: 1;
    }
    .ts-branch-badge {
      font-size: 9px;
      font-weight: 600;
      color: var(--primary);
      background: rgba(107,124,255,0.1);
      padding: 1px 5px;
      border-radius: var(--radius-pill);
    }
    .ts-loading {
      font-size: 12px;
      color: var(--text-tertiary);
      padding: 16px 0;
    }
  `;
  document.head.appendChild(s);
}
