import { For, Show, createEffect, createMemo, createSignal, on } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import { Markdown } from "./Markdown";
import { ToolUseCard, ToolUseStack } from "./ToolUseCard";
import { ThinkingBlock } from "./ThinkingBlock";
import { ThreadSetup } from "./ThreadSetup";
import { LifecycleBanner } from "./LifecycleBanner";
import { McpPanel } from "../sidebar/McpPanel";
import { ThemeSelector } from "../settings/ThemeSelector";
import { SearchOverlay } from "../shared/SearchOverlay";
import { SkillsPanel } from "../skills/SkillsPanel";
import type { ContentBlock } from "../../types";

// Split text into alternating text/link parts for clickable URL rendering
function linkifyParts(text: string): Array<{ type: "text" | "link"; value: string }> {
  const urlRegex = /(https?:\/\/[^\s<>"'`]+)/g;
  const parts: Array<{ type: "text" | "link"; value: string }> = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = urlRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push({ type: "text", value: text.slice(lastIndex, match.index) });
    }
    // Trim trailing punctuation that's unlikely to be part of the URL
    let url = match[0];
    const trailing = url.match(/[.,;:!?)\]}>]+$/);
    if (trailing) {
      url = url.slice(0, url.length - trailing[0].length);
    }
    parts.push({ type: "link", value: url });
    lastIndex = match.index + url.length;
  }
  if (lastIndex < text.length) {
    parts.push({ type: "text", value: text.slice(lastIndex) });
  }
  if (parts.length === 0) parts.push({ type: "text", value: text });
  return parts;
}

export function ChatArea() {
  const { store, approveRequest, denyRequest, addProject, newThread } = appStore;
  let scrollRef: HTMLDivElement | undefined;

  const messages = () => {
    if (!store.activeTab) return [];
    return store.threadMessages[store.activeTab] || [];
  };

  const isGenerating = () => {
    if (!store.activeTab) return false;
    return store.runStates[store.activeTab] === "generating";
  };

  const isLoadingMessages = () => {
    if (!store.activeTab) return false;
    return !!store.threadMessagesLoading[store.activeTab];
  };

  const worktree = () => store.worktrees[store.activeTab || ""] || null;

  // Core derived state — must be declared early (used by needsGitInit, linkedPr, etc.)
  const activeProject = createMemo(() => {
    if (!store.activeTab) return null;
    return store.projects.find((p) => p.threads.some((t) => t.id === store.activeTab)) || null;
  });

  const hasFolder = createMemo(() => {
    const proj = activeProject();
    return proj != null && proj.path !== ".";
  });

  const isGitProject = createMemo(() => {
    const proj = activeProject();
    if (!proj || proj.path === ".") return false;
    const status = store.projectGitStatus[proj.id];
    return status === "git" || status === "github";
  });

  const isGithubProject = createMemo(() => {
    const proj = activeProject();
    if (!proj) return false;
    return store.projectGitStatus[proj.id] === "github";
  });

  // Inline confirmation for destructive git actions
  const [confirmingMerge, setConfirmingMerge] = createSignal(false);
  let confirmTimer: ReturnType<typeof setTimeout> | undefined;

  function requestMergeConfirm() {
    setConfirmingMerge(true);
    if (confirmTimer) clearTimeout(confirmTimer);
    confirmTimer = setTimeout(() => setConfirmingMerge(false), 4000);
  }

  function confirmAndMerge() {
    setConfirmingMerge(false);
    if (confirmTimer) clearTimeout(confirmTimer);
    handleMergeWorktree();
  }

  // Track whether the worktree branch is up-to-date with remote
  const [syncStatus, setSyncStatus] = createSignal<string>("clean"); // "clean" | "dirty" | "ahead" | "diverged"
  const prIsClean = () => syncStatus() === "clean";

  // Check sync status when thread changes or worktree is active
  createEffect(() => {
    const tab = store.activeTab;
    if (!tab) return;
    const wt = store.worktrees[tab];
    if (!wt || !wt.active) return;
    const pr = store.projectPrMap;
    // Debounce: check after a short delay
    setTimeout(async () => {
      try {
        const status = await ipc.checkWorktreeSyncStatus(tab);
        setSyncStatus(status);
      } catch {
        setSyncStatus("clean");
      }
    }, 500);
  });

  const [creatingPr, setCreatingPr] = createSignal(false);
  const [initializingGit, setInitializingGit] = createSignal(false);
  const [showPrPicker, setShowPrPicker] = createSignal(false);
  const [availablePrs, setAvailablePrs] = createSignal<ipc.OpenPr[]>([]);
  const [loadingPrs, setLoadingPrs] = createSignal(false);
  const [linkingPr, setLinkingPr] = createSignal(false);

  async function openPrPicker() {
    const proj = activeProject();
    if (!proj) return;
    setShowPrPicker(true);
    setLoadingPrs(true);
    try {
      const prs = await ipc.listOpenPrs(proj.path);
      setAvailablePrs(prs);
    } catch {
      setAvailablePrs([]);
    } finally {
      setLoadingPrs(false);
    }
  }

  async function linkPrToThread(pr: ipc.OpenPr) {
    const tab = store.activeTab;
    const proj = activeProject();
    if (!tab || !proj) return;

    // Check if another thread already owns this PR
    try {
      const existing = await ipc.findThreadForPr(pr.number);
      if (existing && existing !== tab) {
        appStore.selectThread(existing);
        setShowPrPicker(false);
        return;
      }
    } catch {}

    setLinkingPr(true);
    try {
      // Checkout PR branch into worktree
      const wt = await ipc.checkoutPrIntoWorktree(tab, pr.number, proj.path, proj.id);
      appStore.setStore("worktrees", tab, wt);
      appStore.setStore("projectPrMap", proj.id, tab, pr.number);

      // Rename thread to PR title
      const title = `PR #${pr.number}: ${pr.title}`;
      await ipc.renameThread(tab, title);
      appStore.setStore("projects", (projects) =>
        projects.map((p) => ({
          ...p,
          threads: p.threads.map((t) => t.id === tab ? { ...t, title } : t),
        }))
      );

      setShowPrPicker(false);

      // Inject context message
      appStore.setStore("threadMessages", tab, (msgs) => [
        ...(msgs || []),
        { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, content: `Linked to PR #${pr.number}: ${pr.title}\nBranch: ${pr.branch} · by @${pr.author}` },
      ]);
    } catch (e) {
      appStore.setStore("threadMessages", tab, (msgs) => [
        ...(msgs || []),
        { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, system_kind: "error" as const, content: `Failed to link PR: ${e}` },
      ]);
    } finally {
      setLinkingPr(false);
    }
  }

  // Whether the active project is a non-git folder that needs initialization
  const needsGitInit = createMemo(() => {
    const proj = activeProject();
    if (!proj || proj.path === ".") return false;
    return store.projectGitStatus[proj.id] === "none";
  });

  // Re-check git status when switching threads (detects external git init)
  createEffect(on(
    () => store.activeTab,
    (tab) => {
      if (!tab) return;
      try {
        const proj = activeProject();
        if (proj && proj.path !== ".") {
          appStore.loadProjectGitStatus(proj.id, proj.path, proj.threads, true);
        }
      } catch (e) {
        console.error("Failed to check git status:", e);
      }
    },
    { defer: true }
  ));

  async function handleInitGit() {
    const proj = activeProject();
    if (!proj) return;
    setInitializingGit(true);
    try {
      await ipc.gitInitRepo(proj.path);
      // Refresh git status
      await appStore.loadProjectGitStatus(proj.id, proj.path, proj.threads, true);
      const tab = store.activeTab;
      if (tab) {
        appStore.setStore("threadMessages", tab, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, content: "Git repository initialized. Your next message will create an isolated branch for this thread." },
        ]);
      }
    } catch (e) {
      const tab = store.activeTab;
      if (tab) {
        appStore.setStore("threadMessages", tab, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, system_kind: "error" as const, content: `Failed to initialize git: ${e}` },
        ]);
      }
    } finally {
      setInitializingGit(false);
    }
  }

  // Check if thread has a linked PR
  const linkedPr = () => {
    const proj = activeProject();
    const tab = store.activeTab;
    if (!proj || !tab) return null;
    const prMap = store.projectPrMap[proj.id];
    return prMap?.[tab] ?? null;
  };

  async function handleCreatePr() {
    const tab = store.activeTab;
    const project = activeProject();
    const wt = store.worktrees[tab || ""];
    if (!tab || !project || !wt?.active) return;

    setCreatingPr(true);
    try {
      const thread = project.threads.find((t) => t.id === tab);
      const title = thread?.title || "Changes from CodeForge";
      const body = `## Summary\nChanges made via CodeForge thread: ${title}\n\n---\n*Created with [CodeForge](https://github.com/codeforge)*`;

      const prUrl = await ipc.createPrFromWorktree(tab, project.path, title, body);

      // Extract PR number and update store
      const prNum = prUrl.split("/").pop();
      if (prNum) {
        appStore.setStore("projectPrMap", project.id, tab, parseInt(prNum, 10));
      }

      appStore.setStore("threadMessages", tab, (msgs) => [
        ...(msgs || []),
        { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, content: `PR created: ${prUrl}` },
      ]);
    } catch (e) {
      appStore.setStore("threadMessages", tab, (msgs) => [
        ...(msgs || []),
        { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, system_kind: "error" as const, content: `Failed to create PR: ${e}` },
      ]);
    } finally {
      setCreatingPr(false);
    }
  }

  function cancelMerge() {
    setConfirmingMerge(false);
    if (confirmTimer) clearTimeout(confirmTimer);
  }

  // Track whether user has manually scrolled away from the bottom
  let userScrolledAway = false;

  function handleScroll() {
    if (!scrollRef) return;
    const distFromBottom = scrollRef.scrollHeight - scrollRef.scrollTop - scrollRef.clientHeight;
    userScrolledAway = distFromBottom > 150;
  }

  // Track content of the last message to trigger scroll during streaming
  const lastMsgContent = () => {
    const msgs = messages();
    if (msgs.length === 0) return 0;
    const last = msgs[msgs.length - 1];
    return last.content?.length || 0;
  };

  // Debounced scroll-to-bottom — avoids layout thrashing during streaming
  let prevMsgCount = 0;
  let scrollDebounce: ReturnType<typeof setTimeout> | null = null;

  function scrollToBottom() {
    if (scrollDebounce) clearTimeout(scrollDebounce);
    scrollDebounce = setTimeout(() => {
      if (scrollRef && !userScrolledAway) {
        scrollRef.scrollTo({ top: scrollRef.scrollHeight, behavior: "smooth" });
      }
    }, 80);
  }

  createEffect(() => {
    const len = messages().length;
    const _g = isGenerating();
    const _c = lastMsgContent();
    if (len !== prevMsgCount) {
      userScrolledAway = false;
      prevMsgCount = len;
    }
    if (!userScrolledAway) {
      scrollToBottom();
    }
  });

  createEffect(() => {
    const tab = store.activeTab;
    if (tab && !store.worktrees[tab]) {
      ipc.getWorktree(tab).then((wt) => {
        if (wt) appStore.setStore("worktrees", tab, wt);
      });
    }
  });

  async function handleCreateWorktree() {
    const tab = store.activeTab;
    if (!tab) return;
    const thread = store.projects.flatMap((p) => p.threads).find((t) => t.id === tab);
    const project = store.projects.find((p) => p.threads.some((t) => t.id === tab));
    if (!thread || !project || project.path === ".") return;
    try {
      const wt = await ipc.createWorktree(tab, thread.title, project.path, project.id);
      appStore.setStore("worktrees", tab, wt);
    } catch (e) {
      console.error("Failed to create worktree:", e);
    }
  }

  const [merging, setMerging] = createSignal(false);

  async function handleMergeWorktree() {
    const tab = store.activeTab;
    if (!tab) return;
    const project = store.projects.find((p) => p.threads.some((t) => t.id === tab));
    if (!project) return;
    setMerging(true);
    try {
      const msg = await ipc.mergeWorktree(tab, project.path);
      // Push to PR only — do NOT mark the thread as merged.
      // Thread stays editable. It only locks when get_pr_status detects the
      // PR has been merged on GitHub.
      appStore.setStore("threadMessages", tab, (msgs) => [
        ...(msgs || []),
        { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, content: msg },
      ]);
    } catch (e) {
      const errorMsg = String(e);

      // Typed divergence error from merge_worktree: we've detected that the
      // remote PR branch has commits we don't. Don't ask the AI to "fix it";
      // surface a clear, actionable message instead.
      if (errorMsg.startsWith("DIVERGED:") || errorMsg.includes("DIVERGED:")) {
        const aheadMatch = errorMsg.match(/ahead=(\d+)/);
        const behindMatch = errorMsg.match(/behind=(\d+)/);
        const ahead = aheadMatch ? aheadMatch[1] : "?";
        const behind = behindMatch ? behindMatch[1] : "?";
        const pretty =
          errorMsg.includes("lease_rejected")
            ? `The remote PR branch moved while we were pushing. Pull first, then try again.`
            : `Your branch has diverged from the PR — ${ahead} ahead, ${behind} behind. Pull the remote changes and resolve any conflicts, then try pushing again.`;
        appStore.setStore("threadMessages", tab, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, system_kind: "warn" as const, content: pretty },
        ]);
        return;
      }

      // Generic failure — auto-prompt the AI to help resolve the conflict.
      const prompt = `My merge/push just failed with this error:\n\n\`\`\`\n${errorMsg}\n\`\`\`\n\nPlease help me resolve this. Check the git status, identify any conflicts, and fix them.`;

      try {
        const msgId = await ipc.persistUserMessage(tab, prompt);
        appStore.setStore("threadMessages", tab, (msgs) => [
          ...(msgs || []),
          { id: msgId, thread_id: tab, role: "user" as const, content: prompt },
        ]);

        const wt = store.worktrees[tab];
        const cwd = wt?.active ? wt.path : (project && project.path !== "." ? project.path : ".");

        appStore.setStore("runStates", tab, "generating");
        await ipc.sendMessage(tab, prompt, store.selectedProvider, cwd, store.selectedModel ?? undefined);
      } catch (sendErr) {
        appStore.setStore("threadMessages", tab, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, system_kind: "error" as const, content: `Merge failed: ${errorMsg}` },
        ]);
      }
    } finally {
      setMerging(false);
    }
  }

  function setSuggestion(text: string) {
    appStore.setStore("composerText", text);
  }

  const isVirtualTab = () => store.activeTab?.startsWith("__") || false;

  return (
    <>
    {/* Lifecycle banner — pinned above the chat scroll, not inside it. */}
    <Show when={store.activeTab && !store.activeTab.startsWith("__")}>
      <LifecycleBanner threadId={store.activeTab!} />
    </Show>
    <div class="chat-area" ref={scrollRef} onScroll={handleScroll}>
      {/* Virtual tab content */}
      <Show when={store.activeTab === "__mcp__"}>
        <div class="virtual-tab-content">
          <McpPanel />
        </div>
      </Show>
      <Show when={store.activeTab === "__themes__"}>
        <div class="virtual-tab-content">
          <ThemeSelector inline />
        </div>
      </Show>
      <Show when={store.activeTab === "__search__"}>
        <div class="virtual-tab-content">
          <SearchOverlay inline />
        </div>
      </Show>
      <Show when={store.activeTab === "__skills__"}>
        <div class="virtual-tab-content">
          <SkillsPanel />
        </div>
      </Show>

      {/* Regular chat content */}
      <Show when={!isVirtualTab()}>
      <Show
        when={store.activeTab}
        fallback={
          <Show when={store.projects.filter((p) => p.path !== ".").length > 0} fallback={
            <div class="chat-empty">
              <div class="onboarding-card">
                <div class="onboarding-icon">
                  <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="url(#og1)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
                    <line x1="12" y1="11" x2="12" y2="17" /><line x1="9" y1="14" x2="15" y2="14" />
                    <defs>
                      <linearGradient id="og1" x1="2" y1="3" x2="22" y2="21" gradientUnits="userSpaceOnUse">
                        <stop stop-color="var(--primary)" /><stop offset="1" stop-color="var(--purple)" />
                      </linearGradient>
                    </defs>
                  </svg>
                </div>
                <h2 class="onboarding-title">Add your first project</h2>
                <p class="onboarding-desc">Point CodeForge to a project folder to get started</p>
                <button class="onboarding-btn" onClick={() => addProject()}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
                    <line x1="12" y1="11" x2="12" y2="17" /><line x1="9" y1="14" x2="15" y2="14" />
                  </svg>
                  Choose a folder
                </button>
                <button class="onboarding-link" onClick={() => newThread()}>
                  Or start without a project
                </button>
              </div>
              <div class="shortcut-hints" style={{ "margin-top": "24px" }}>
                <span class="kbd-hint"><kbd>&#8984;K</kbd> Command palette</span>
                <span class="kbd-hint"><kbd>&#8984;N</kbd> New thread</span>
              </div>
            </div>
          }>
            <div class="chat-empty">
              <div class="hero-mark">
                <svg width="40" height="40" viewBox="0 0 56 56" fill="none">
                  <path d="M12 10L6 28L12 46" stroke="url(#hg1)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                  <path d="M44 10L50 28L44 46" stroke="url(#hg1)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                  <path d="M33 8L23 48" stroke="url(#hg2)" stroke-width="2" stroke-linecap="round" />
                  <defs>
                    <linearGradient id="hg1" x1="6" y1="10" x2="50" y2="46" gradientUnits="userSpaceOnUse">
                      <stop stop-color="var(--primary)" /><stop offset="1" stop-color="var(--purple)" />
                    </linearGradient>
                    <linearGradient id="hg2" x1="23" y1="48" x2="33" y2="8" gradientUnits="userSpaceOnUse">
                      <stop stop-color="var(--primary)" /><stop offset="1" stop-color="var(--pink)" />
                    </linearGradient>
                  </defs>
                </svg>
              </div>
              <h2 class="hero-title">CodeForge</h2>
              <p class="hero-subtitle">Select a thread from the sidebar, or create a new one</p>
              <div class="hero-quick-actions">
                <For each={store.projects.filter((p) => p.path !== ".").slice(0, 3)}>
                  {(project) => (
                    <button class="hero-quick-btn" onClick={() => newThread(project.id)}>
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                        <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
                      </svg>
                      New Thread in {project.name}
                    </button>
                  )}
                </For>
              </div>
              <div class="shortcut-hints">
                <span class="kbd-hint"><kbd>&#8984;K</kbd> Command palette</span>
                <span class="kbd-hint"><kbd>&#8984;N</kbd> New thread</span>
                <span class="kbd-hint"><kbd>&#8984;O</kbd> Add project</span>
              </div>
              <div class="shortcut-hints" style={{ "margin-top": "4px" }}>
                <span class="kbd-hint"><kbd>&#8984;\</kbd> Split view</span>
                <span class="kbd-hint"><kbd>&#8984;&#8679;D</kbd> Diff panel</span>
                <span class="kbd-hint"><kbd>&#8984;&#8679;B</kbd> Browser</span>
                <span class="kbd-hint"><kbd>&#8984;&#8679;?</kbd> All shortcuts</span>
              </div>
            </div>
          </Show>
        }
      >
        {/* Worktree banner moved to WorktreeBanner component below — rendered outside scroll */}

        {/* Loading skeleton while messages are being fetched */}
        <Show when={isLoadingMessages()}>
          <div class="messages-loading">
            <div class="msg-skeleton">
              <div class="skeleton-line wide" />
              <div class="skeleton-line" />
            </div>
            <div class="msg-skeleton assistant">
              <div class="skeleton-line wide" />
              <div class="skeleton-line medium" />
              <div class="skeleton-line" />
            </div>
          </div>
        </Show>

        <Show
          when={(messages().length > 0 || isGenerating()) && !isLoadingMessages()}
          fallback={
            <div class="chat-empty">
              {/* Context-aware greeting based on thread state */}
              <Show when={linkedPr()}>
                <div class="chat-empty-context">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M13 6h3a2 2 0 012 2v7" /><line x1="6" y1="9" x2="6" y2="21" />
                  </svg>
                  <span>PR #{linkedPr()} · Ready to review or continue work</span>
                </div>
              </Show>
              <Show when={!linkedPr() && isGitProject()}>
                <div class="chat-empty-context">
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--primary)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M6 21V9a9 9 0 009 9" />
                  </svg>
                  <span>Your first message creates an isolated branch for this thread</span>
                </div>
              </Show>

              <p class="new-convo">What do you want to build?</p>
              <p class="provider-hint">
                {store.selectedProvider === "claude_code" ? "Claude Code" : "Codex"} · {activeProject()?.name || "No project"}
              </p>

              {/* "Work on PR" action for GitHub projects */}
              <Show when={isGithubProject() && !linkedPr() && !showPrPicker()}>
                <button class="chat-empty-pr-btn" onClick={openPrPicker}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M13 6h3a2 2 0 012 2v7" /><line x1="6" y1="9" x2="6" y2="21" />
                  </svg>
                  Work on a Pull Request
                </button>
              </Show>

              {/* PR picker inline */}
              <Show when={showPrPicker()}>
                <div class="pr-picker">
                  <div class="pr-picker-header">
                    <button class="pr-picker-back" onClick={() => setShowPrPicker(false)}>
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6" /></svg>
                      Back
                    </button>
                    <span class="pr-picker-title">Select a Pull Request</span>
                  </div>
                  <Show when={loadingPrs()}>
                    <div class="pr-picker-loading">Loading PRs...</div>
                  </Show>
                  <Show when={!loadingPrs() && availablePrs().length === 0}>
                    <div class="pr-picker-empty">No open PRs found</div>
                  </Show>
                  <div class="pr-picker-list">
                    <For each={availablePrs()}>
                      {(pr) => (
                        <button class="pr-picker-item" onClick={() => linkPrToThread(pr)} disabled={linkingPr()}>
                          <span class="pr-picker-num">#{pr.number}</span>
                          <span class="pr-picker-info">
                            <span class="pr-picker-pr-title">{pr.title}</span>
                            <span class="pr-picker-meta">{pr.branch} · @{pr.author}</span>
                          </span>
                        </button>
                      )}
                    </For>
                  </div>
                </div>
              </Show>

              <Show when={!showPrPicker()}>
              <div class="suggestion-chips">
                {/* PR-specific suggestions */}
                <Show when={linkedPr()}>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Review this PR and identify issues, bugs, or improvements")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg>
                    Review PR
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Summarize all changes in this PR")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" /><polyline points="14 2 14 8 20 8" /><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" /></svg>
                    Summarize changes
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Address the reviewer feedback on this PR")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" /></svg>
                    Address feedback
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Check for bugs, security issues, or performance problems in this PR")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
                    Check for issues
                  </button>
                </Show>
                {/* Git project suggestions (no PR) */}
                <Show when={!linkedPr() && isGitProject()}>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Build a new feature: ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" /></svg>
                    Build a feature
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Help me fix a bug in ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
                    Fix a bug
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Refactor this code to be cleaner: ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6" /><polyline points="8 6 2 12 8 18" /></svg>
                    Refactor code
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Write tests for ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="9 11 12 14 22 4" /><path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11" /></svg>
                    Write tests
                  </button>
                </Show>
                {/* Non-git project suggestions */}
                <Show when={!isGitProject() && hasFolder()}>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Explain this codebase and its architecture")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 014 4v14a3 3 0 00-3-3H2z" /><path d="M22 3h-6a4 4 0 00-4 4v14a3 3 0 013-3h7z" /></svg>
                    Explain codebase
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Help me fix a bug in ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
                    Fix a bug
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Write tests for ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="9 11 12 14 22 4" /><path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11" /></svg>
                    Write tests
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Refactor this code to be cleaner: ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6" /><polyline points="8 6 2 12 8 18" /></svg>
                    Refactor
                  </button>
                </Show>
                {/* Uncategorized thread (no project) */}
                <Show when={!hasFolder()}>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Explain this codebase and its architecture")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 014 4v14a3 3 0 00-3-3H2z" /><path d="M22 3h-6a4 4 0 00-4 4v14a3 3 0 013-3h7z" /></svg>
                    Explain codebase
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Help me write ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9" /><path d="M16.5 3.5a2.12 2.12 0 013 3L7 19l-4 1 1-4z" /></svg>
                    Help me write code
                  </button>
                </Show>
              </div>
              </Show>
            </div>
          }
        >
          <div class="messages-container">
            <For each={messages()}>
              {(msg, idx) => (
                <MessageBubble
                  msg={msg}
                  isGenerating={isGenerating()}
                  isLast={idx() === messages().length - 1}
                  isLastUser={msg.role === "user" && (() => {
                    const msgs = messages();
                    for (let i = msgs.length - 1; i >= 0; i--) {
                      if (msgs[i].role === "user") return msgs[i].id === msg.id;
                    }
                    return false;
                  })()}
                  threadId={store.activeTab!}
                />
              )}
            </For>

            {/* Suggestion chips removed — now shown in the empty state above */}

            <For each={store.pendingApprovals.filter((a) => a.threadId === store.activeTab)}>
              {(approval) => {
                const toolName = () => approval.description.split(":")[0]?.trim() || "tool";

                function handleAutoAccept() {
                  // Set permission mode to bypass and approve ALL pending
                  ipc.setSetting("permission_mode", "bypassPermissions").catch(() => {});
                  appStore.setStore("autoAcceptEnabled", true);
                  appStore.persistState();
                  // Approve all pending approvals for this thread
                  const pending = [...store.pendingApprovals.filter((a) => a.threadId === approval.threadId)];
                  for (const a of pending) {
                    approveRequest(a);
                  }
                }

                return (
                  <div class="approval-card">
                    <div class="approval-header">
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--amber)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
                        <line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
                      </svg>
                      <span class="approval-title">Permission Required</span>
                      <span class="approval-tool">{toolName()}</span>
                    </div>
                    <pre class="approval-desc">{approval.description}</pre>
                    <div class="approval-actions">
                      <button class="deny-btn" onClick={() => denyRequest(approval)}>Deny</button>
                      <button class="approve-btn" onClick={() => approveRequest(approval)}>Approve</button>
                      <button class="bypass-btn" onClick={handleAutoAccept} title="Auto-approve everything from now on">
                        Auto Accept
                      </button>
                    </div>
                  </div>
                );
              }}
            </For>

            <Show when={isGenerating() && messages().length === 0}>
              <div class="typing-shimmer">
                <div class="typing-shimmer-line" />
                <div class="typing-shimmer-line short" />
              </div>
            </Show>
          </div>
        </Show>
      </Show>
      </Show>
    </div>

    {/* Git migration banner — shown for non-git project folders */}
    <Show when={needsGitInit() && hasFolder() && messages().length > 0}>
      <div class="git-migration-bar">
        <svg class="gm-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
        <span class="gm-text">
          This folder isn't a git repo yet. Initialize git to get isolated branches per thread.
        </span>
        <button
          class="gm-init-btn"
          onClick={handleInitGit}
          disabled={initializingGit()}
        >
          {initializingGit() ? "Initializing..." : "Initialize Git"}
        </button>
      </div>
    </Show>

    {/* Worktree bar — persistent, hidden after merge */}
    <Show when={isGitProject() && worktree()?.active}>
      <div class="worktree-bar">
        <svg class="wt-bar-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="6" y1="3" x2="6" y2="15" /><circle cx="18" cy="6" r="3" /><circle cx="6" cy="18" r="3" /><path d="M18 9a9 9 0 01-9 9" />
        </svg>
        <span class="wt-bar-branch">{worktree()!.branch}</span>
        <span class="wt-bar-sep">&middot;</span>
        <span class="wt-bar-path">{worktree()!.path}</span>
        <span class="wt-bar-spacer" />

        {/* Create PR button — shown when no PR is linked AND the thread has actual assistant activity */}
        <Show when={!linkedPr() && messages().some((m) => m.role === "assistant")}>
          <button
            class="wt-bar-create-pr"
            onClick={handleCreatePr}
            disabled={creatingPr()}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M13 6h3a2 2 0 012 2v7" /><line x1="6" y1="9" x2="6" y2="21" />
            </svg>
            {creatingPr() ? "Creating..." : "Create PR"}
          </button>
        </Show>

        {/* Push to PR button — only after at least one assistant message */}
        <Show when={linkedPr() && messages().some((m) => m.role === "assistant")}>
          <Show when={merging()}>
            <button class="wt-bar-action" disabled>
              <span class="wt-spinner" /> Pushing...
            </button>
          </Show>
          <Show when={!merging()}>
            <Show when={confirmingMerge()} fallback={
              <button class="wt-bar-action" onClick={requestMergeConfirm}>
                Push to PR #{linkedPr()}
              </button>
            }>
              <div class="wt-confirm-group">
                <span class="wt-confirm-label">Are you sure?</span>
                <button class="wt-confirm-yes" onClick={confirmAndMerge}>Push</button>
                <button class="wt-confirm-no" onClick={cancelMerge}>Cancel</button>
              </div>
            </Show>
          </Show>
        </Show>

      </div>
    </Show>

    <style>{`
    .chat-area {
      flex: 1;
      overflow-y: auto;
      display: flex;
      flex-direction: column;
    }

    /* ── Virtual tab content ── */
    .virtual-tab-content {
      flex: 1;
      overflow-y: auto;
      padding: 16px;
      animation: fade-in 0.15s ease both;
    }
    .virtual-tab-placeholder {
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-tertiary);
      font-size: 14px;
    }

    /* ── Empty states ── */
    .chat-empty {
      flex: 1;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      color: var(--text-secondary);
      gap: 8px;
      animation: fade-in 0.2s ease both;
    }
    .hero-mark {
      margin-bottom: 8px;
      opacity: 0.5;
    }
    .hero-title {
      font-size: 28px;
      font-weight: 700;
      letter-spacing: -0.8px;
      color: var(--text);
    }
    .hero-subtitle {
      font-size: 13px;
      color: var(--text-tertiary);
      margin-bottom: 16px;
    }
    .shortcut-hints {
      display: flex;
      flex-wrap: wrap;
      gap: 8px 20px;
      justify-content: center;
      max-width: 520px;
    }
    .kbd-hint {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 11px;
      color: var(--text-tertiary);
    }
    .kbd-hint kbd {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      padding: 2px 6px;
      background: var(--bg-accent);
      border: 1px solid var(--border-strong);
      border-radius: 4px;
      font-size: 10px;
      font-family: var(--font-body);
      color: var(--text-secondary);
      line-height: 1.4;
    }

    /* ── Onboarding card (no projects) ── */
    .onboarding-card {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 8px;
      padding: 40px 48px;
      background: var(--bg-card);
      border: 1px solid var(--border-strong);
      border-radius: 16px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.3);
      animation: cmdSlideIn 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    }
    @keyframes cmdSlideIn {
      from { opacity: 0; transform: translateY(-6px) scale(0.98); }
      to { opacity: 1; transform: translateY(0) scale(1); }
    }
    .onboarding-icon {
      margin-bottom: 4px;
      opacity: 0.9;
    }
    .onboarding-title {
      font-size: 22px;
      font-weight: 700;
      letter-spacing: -0.5px;
      color: var(--text);
    }
    .onboarding-desc {
      font-size: 13px;
      color: var(--text-tertiary);
      margin-bottom: 8px;
    }
    .onboarding-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      padding: 10px 28px;
      font-size: 14px;
      font-weight: 600;
      color: #fff;
      background: var(--primary);
      border-radius: var(--radius-md);
      transition: filter 0.15s, transform 0.1s;
      margin-top: 4px;
    }
    .onboarding-btn:hover { filter: brightness(1.15); }
    .onboarding-btn:active { transform: scale(0.97); }
    .onboarding-link {
      font-size: 12px;
      color: var(--text-tertiary);
      background: none;
      border: none;
      cursor: pointer;
      padding: 6px 12px;
      transition: color 0.15s;
      text-decoration: underline;
      text-underline-offset: 2px;
    }
    .onboarding-link:hover { color: var(--text-secondary); }

    /* ── Hero quick actions (projects exist, no tab selected) ── */
    .hero-quick-actions {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 8px;
      margin-bottom: 12px;
      width: 100%;
      max-width: 520px;
    }
    .hero-quick-btn {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 16px;
      font-size: 13px;
      font-weight: 500;
      color: var(--text-secondary);
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      transition: all 0.15s ease;
      cursor: pointer;
      width: 100%;
      text-align: left;
    }
    .hero-quick-btn svg { color: var(--primary); flex-shrink: 0; }
    .hero-quick-btn:hover {
      background: rgba(107, 124, 255, 0.08);
      border-color: rgba(107, 124, 255, 0.3);
      color: var(--text);
      transform: translateY(-1px);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    }

    /* ── "Work on PR" button ── */
    .chat-empty-pr-btn {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 10px 20px;
      background: var(--bg-card);
      border: 1px solid rgba(76, 214, 148, 0.25);
      border-radius: var(--radius-md);
      font-size: 13px;
      font-weight: 600;
      color: var(--green);
      cursor: pointer;
      transition: all 0.15s;
      margin-bottom: 8px;
    }
    .chat-empty-pr-btn:hover {
      background: rgba(76, 214, 148, 0.08);
      border-color: rgba(76, 214, 148, 0.4);
      transform: translateY(-1px);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    }

    /* ── Inline PR picker ── */
    .pr-picker {
      width: 100%;
      max-width: 520px;
      background: var(--bg-card);
      border: 1px solid var(--border-strong);
      border-radius: var(--radius-md);
      overflow: hidden;
      animation: fade-slide-up 0.15s ease both;
    }
    .pr-picker-header {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 10px 14px;
      border-bottom: 1px solid var(--border);
    }
    .pr-picker-back {
      display: flex;
      align-items: center;
      gap: 4px;
      font-size: 11px;
      color: var(--text-tertiary);
      transition: color 0.12s;
    }
    .pr-picker-back:hover { color: var(--text-secondary); }
    .pr-picker-title {
      font-size: 12px;
      font-weight: 600;
      color: var(--text);
    }
    .pr-picker-loading, .pr-picker-empty {
      padding: 20px;
      text-align: center;
      font-size: 12px;
      color: var(--text-tertiary);
    }
    .pr-picker-list {
      max-height: 280px;
      overflow-y: auto;
    }
    .pr-picker-item {
      display: flex;
      align-items: flex-start;
      gap: 10px;
      width: 100%;
      padding: 10px 14px;
      text-align: left;
      transition: background 0.1s;
      cursor: pointer;
    }
    .pr-picker-item:hover { background: var(--bg-hover); }
    .pr-picker-item + .pr-picker-item { border-top: 1px solid var(--border); }
    .pr-picker-item:disabled { opacity: 0.5; cursor: default; }
    .pr-picker-num {
      font-size: 12px;
      font-family: var(--font-mono);
      font-weight: 600;
      color: var(--green);
      flex-shrink: 0;
      min-width: 40px;
    }
    .pr-picker-info {
      display: flex;
      flex-direction: column;
      gap: 2px;
      min-width: 0;
    }
    .pr-picker-pr-title {
      font-size: 12px;
      font-weight: 500;
      color: var(--text);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .pr-picker-meta {
      font-size: 10px;
      font-family: var(--font-mono);
      color: var(--text-tertiary);
    }

    .chat-empty-context {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 6px 14px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-pill);
      font-size: 12px;
      color: var(--text-secondary);
      margin-bottom: 8px;
    }
    .chat-empty-context svg { flex-shrink: 0; }
    .chat-empty .new-convo {
      font-size: 18px;
      font-weight: 600;
      color: var(--text);
      letter-spacing: -0.3px;
    }
    .chat-empty .provider-hint {
      font-size: 12px;
      color: var(--text-tertiary);
      margin-bottom: 12px;
    }

    /* ── Suggestion chips — 2-column grid ── */
    .suggestion-chips {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 8px;
      margin-top: 12px;
      width: 100%;
      max-width: 520px;
    }
    .suggestion-chip {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 14px;
      font-size: 12px;
      font-weight: 500;
      color: var(--text-secondary);
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      cursor: pointer;
      transition: all 0.15s ease;
      text-align: left;
    }
    .suggestion-chip svg { color: var(--text-tertiary); flex-shrink: 0; transition: color 0.15s; }
    .suggestion-chip:hover {
      background: rgba(107, 124, 255, 0.06);
      border-color: rgba(107, 124, 255, 0.25);
      color: var(--text);
      transform: translateY(-1px);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    }
    .suggestion-chip:hover svg { color: var(--primary); }
    @media (max-width: 500px) {
      .suggestion-chips { grid-template-columns: 1fr; }
      .hero-quick-actions { grid-template-columns: 1fr; }
    }

    /* ── Messages ── */
    .messages-container {
      max-width: 768px;
      width: 100%;
      margin: 0 auto;
      padding: 16px 16px 24px;
      display: flex;
      flex-direction: column;
      gap: 20px;
      content-visibility: auto;
      contain-intrinsic-size: auto 500px;
      animation: fade-in 0.15s ease both;
    }

    /* ── Loading skeleton ── */
    .messages-loading {
      max-width: 768px;
      width: 100%;
      margin: 0 auto;
      padding: 24px 16px;
      display: flex;
      flex-direction: column;
      gap: 20px;
    }
    .msg-skeleton {
      display: flex;
      flex-direction: column;
      gap: 8px;
      max-width: 65%;
      margin-left: auto;
    }
    .msg-skeleton.assistant {
      margin-left: 0;
      margin-right: auto;
      max-width: 80%;
    }
    .skeleton-line {
      height: 12px;
      background: var(--bg-accent);
      border-radius: 6px;
      width: 40%;
      animation: skeleton-pulse 1.2s ease-in-out infinite;
    }
    .skeleton-line.wide { width: 90%; }
    .skeleton-line.medium { width: 65%; }
    @keyframes skeleton-pulse {
      0%, 100% { opacity: 0.4; }
      50% { opacity: 0.8; }
    }

    /* ── Direction C: Directional message entry ── */
    .msg {
      width: 100%;
    }
    .msg-user {
      animation: msg-user-in 0.2s cubic-bezier(0.16, 1, 0.3, 1) both;
    }
    .msg-assistant, .msg-system {
      animation: msg-assistant-in 0.2s cubic-bezier(0.16, 1, 0.3, 1) both;
    }
    @keyframes msg-user-in {
      from { opacity: 0; transform: translateX(8px); }
      to { opacity: 1; transform: translateX(0); }
    }
    @keyframes msg-assistant-in {
      from { opacity: 0; transform: translateX(-6px); }
      to { opacity: 1; transform: translateX(0); }
    }

    /* ── User message ── */
    .msg-user-bubble {
      max-width: 85%;
      margin-left: auto;
      padding: 10px 14px;
      border-radius: var(--radius-lg);
      font-size: 14px;
      line-height: 1.6;
      white-space: pre-wrap;
      word-break: break-word;
      background: rgba(107, 124, 255, 0.08);
      border: 1px solid rgba(107, 124, 255, 0.12);
      color: var(--text);
    }

    /* ── System message ── */
    /* Base pill (info variant). Other variants override color/background. */
    .msg-system-pill {
      text-align: center;
      background: var(--bg-muted);
      border: 1px solid var(--border);
      font-size: 12px;
      color: var(--text-secondary);
      border-radius: var(--radius-pill);
      padding: 4px 12px;
      display: inline-block;
      margin: 0 auto;
      max-width: 80%;
      animation: fade-in 0.15s ease both;
      line-height: 1.4;
    }
    .msg-system-pill--info {
      /* default — muted grey */
    }
    .msg-system-pill--warn {
      color: var(--amber, #e6b84d);
      background: rgba(240, 184, 64, 0.10);
      border-color: rgba(240, 184, 64, 0.35);
    }
    .msg-system-pill--error {
      color: var(--red);
      background: rgba(242, 95, 103, 0.10);
      border-color: rgba(242, 95, 103, 0.35);
    }
    .msg-system-pill--review {
      color: var(--text);
      background: rgba(102, 184, 224, 0.08);
      border-color: rgba(102, 184, 224, 0.35);
      text-align: left;
      padding: 8px 14px;
      max-width: 90%;
      border-radius: var(--radius-md);
      white-space: pre-wrap;
    }
    .msg-system-link {
      color: var(--primary);
      text-decoration: underline;
      text-underline-offset: 2px;
      font-weight: 500;
      cursor: pointer;
    }
    .msg-system-link:hover { filter: brightness(1.15); }
    .msg-system { text-align: center; }

    /* ── Assistant message ── */
    .msg-assistant {
      width: 100%;
      display: flex;
      flex-direction: column;
      gap: 0;
    }

    /* Direction A: Streaming text lines fade in with slide-up */
    .msg-body p, .msg-body li, .msg-body pre, .msg-body blockquote, .msg-body h1, .msg-body h2, .msg-body h3, .msg-body h4 {
      animation: streaming-line-in 0.15s ease-out both;
    }
    @keyframes streaming-line-in {
      from { opacity: 0; transform: translateY(4px); }
      to { opacity: 1; transform: translateY(0); }
    }

    /* Direction A: Tool cards stagger in from left */
    .tc, .tc-stack {
      animation: tool-card-in 0.2s cubic-bezier(0.16, 1, 0.3, 1) both;
    }
    @keyframes tool-card-in {
      from { opacity: 0; transform: translateX(-8px); }
      to { opacity: 1; transform: translateX(0); }
    }

    /* Details toggle */
    .msg-details-toggle {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      font-size: 11px;
      font-weight: 500;
      color: var(--text-tertiary);
      padding: 3px 8px;
      border-radius: var(--radius-pill);
      margin-bottom: 6px;
      transition: color 0.12s, background 0.12s;
      align-self: flex-start;
    }
    .msg-details-toggle:hover {
      color: var(--text-secondary);
      background: var(--bg-hover);
    }
    .msg-details-toggle svg { flex-shrink: 0; }

    /* Message body — full width, clean typography */
    .msg-body {
      font-size: 14px;
      line-height: 1.6;
      color: var(--text);
    }
    .msg-no-text {
      font-size: 12px;
      color: var(--text-tertiary);
      font-style: italic;
    }

    /* Footer — right-aligned meta */
    .msg-footer {
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 10px;
      margin-top: 6px;
      min-height: 20px;
    }
    .msg-meta {
      display: flex;
      align-items: center;
      gap: 0;
    }
    .msg-meta-tag {
      font-size: 10px;
      font-weight: 500;
      color: var(--text-tertiary);
      font-family: var(--font-mono);
      white-space: nowrap;
      opacity: 0.7;
    }
    .msg-meta-tag + .msg-meta-tag::before {
      content: "·";
      margin: 0 6px;
      opacity: 0.4;
    }
    .msg-copy {
      color: var(--text-tertiary);
      opacity: 0.5;
      padding: 3px;
      border-radius: var(--radius-sm);
      transition: all 0.12s;
      display: flex;
      align-items: center;
    }
    .msg-copy:hover { opacity: 1; background: var(--bg-accent); color: var(--text-secondary); }

    /* ── Message action buttons (edit, retry, regenerate) ── */
    .msg-user-actions {
      display: flex;
      justify-content: flex-end;
      gap: 4px;
      margin-top: 4px;
      opacity: 0;
      transition: opacity 0.15s ease;
    }
    .msg-user-bubble:hover + .msg-user-actions,
    .msg-user-actions:hover {
      opacity: 1;
    }
    .msg:hover .msg-user-actions {
      opacity: 1;
    }
    .msg:hover .msg-action-btn.footer-action {
      opacity: 1;
    }
    .msg-action-btn {
      display: flex;
      align-items: center;
      padding: 3px;
      border-radius: var(--radius-sm);
      color: var(--text-tertiary);
      opacity: 0;
      transition: opacity 0.12s ease, background 0.12s ease, color 0.12s ease;
      cursor: pointer;
    }
    .msg:hover .msg-action-btn {
      opacity: 0.5;
    }
    .msg:hover .msg-action-btn:hover {
      opacity: 1;
      background: var(--bg-accent);
      color: var(--text-secondary);
    }

    /* Direction C: Shimmer typing indicator */
    .typing-shimmer {
      display: flex;
      flex-direction: column;
      gap: 6px;
      padding: 16px 0;
      max-width: 200px;
    }
    .typing-shimmer-line {
      height: 3px;
      border-radius: 2px;
      background: linear-gradient(
        90deg,
        rgba(107, 124, 255, 0.08) 0%,
        rgba(107, 124, 255, 0.25) 40%,
        rgba(107, 124, 255, 0.08) 80%
      );
      background-size: 200% 100%;
      animation: shimmer-flow 1.5s ease-in-out infinite;
    }
    .typing-shimmer-line.short {
      width: 60%;
      animation-delay: 0.15s;
    }
    @keyframes shimmer-flow {
      from { background-position: 200% 0; }
      to { background-position: -200% 0; }
    }
    /* ── Approvals ── */
    @keyframes approval-pulse {
      0%, 100% { border-color: rgba(240, 184, 64, 0.4); box-shadow: 0 0 0 0 rgba(240, 184, 64, 0); }
      50% { border-color: rgba(240, 184, 64, 0.7); box-shadow: 0 0 12px -2px rgba(240, 184, 64, 0.15); }
    }
    .approval-card {
      background: rgba(240, 184, 64, 0.04);
      border: 1px solid rgba(240, 184, 64, 0.4);
      border-radius: var(--radius-md);
      padding: 14px 16px;
      animation: approval-enter 0.3s cubic-bezier(0.16, 1, 0.3, 1) both,
                 approval-pulse 2s ease-in-out 0.3s infinite;
    }
    @keyframes approval-enter {
      from { opacity: 0; transform: scale(0.95) translateY(6px); }
      to { opacity: 1; transform: scale(1) translateY(0); }
    }
    .approval-header {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-bottom: 8px;
    }
    .approval-title {
      font-size: 13px;
      font-weight: 600;
      color: var(--amber);
    }
    .approval-desc {
      font-size: 12px;
      font-family: var(--font-mono);
      color: var(--text-secondary);
      margin-bottom: 12px;
      white-space: pre-wrap;
      overflow-wrap: break-word;
      background: var(--bg-base);
      padding: 8px 10px;
      border-radius: var(--radius-sm);
      max-height: 200px;
      overflow-y: auto;
      margin: 0 0 12px;
    }
    .approval-tool {
      font-size: 11px;
      font-family: var(--font-mono);
      color: var(--text-tertiary);
      margin-left: auto;
      padding: 1px 6px;
      background: var(--bg-accent);
      border-radius: var(--radius-pill);
    }
    .approval-actions { display: flex; gap: 6px; justify-content: flex-end; flex-wrap: wrap; }
    .approve-btn, .deny-btn, .bypass-btn {
      padding: 6px 12px;
      border-radius: var(--radius-sm);
      font-size: 11px;
      font-weight: 600;
      transition: all 0.12s;
    }
    .approve-btn {
      background: var(--green);
      color: #fff;
    }
    .approve-btn:hover { filter: brightness(1.1); transform: translateY(-1px); }
    .bypass-btn {
      background: rgba(240, 184, 64, 0.1);
      border: 1px solid rgba(240, 184, 64, 0.2);
      color: var(--amber);
    }
    .bypass-btn:hover {
      background: rgba(240, 184, 64, 0.18);
      border-color: rgba(240, 184, 64, 0.35);
    }
    .deny-btn {
      background: var(--bg-muted);
      border: 1px solid var(--border);
      color: var(--text-secondary);
    }
    .deny-btn:hover { border-color: var(--border-strong); color: var(--text); }

    /* ── Persistent worktree bar (below composer + status bar) ── */
    .worktree-bar {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 6px 16px;
      background: rgba(107, 124, 255, 0.06);
      border-top: 1px solid rgba(107, 124, 255, 0.12);
      flex-shrink: 0;
      font-size: 11px;
      order: 99;
    }
    .wt-bar-icon { color: var(--primary); flex-shrink: 0; }
    .wt-bar-branch {
      color: var(--primary);
      font-family: var(--font-mono);
      font-size: 11px;
      font-weight: 600;
      white-space: nowrap;
    }
    .wt-bar-sep { color: var(--text-tertiary); opacity: 0.4; }
    .wt-bar-path {
      color: var(--text-tertiary);
      font-family: var(--font-mono);
      font-size: 10px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      min-width: 0;
    }
    .wt-bar-spacer { flex: 1; }
    .wt-bar-action {
      padding: 4px 12px;
      background: var(--primary);
      color: white;
      border-radius: var(--radius-sm);
      font-size: 11px;
      font-weight: 600;
      transition: filter 0.12s;
      flex-shrink: 0;
    }
    .wt-bar-action:hover { filter: brightness(1.15); }
    .wt-bar-action:disabled { opacity: 0.7; cursor: wait; filter: none; }
    .wt-bar-action:disabled .wt-spinner,
    .wt-bar-action-secondary:disabled .wt-spinner {
      display: inline-block;
      width: 10px;
      height: 10px;
      border: 2px solid rgba(255,255,255,0.3);
      border-top-color: white;
      border-radius: 50%;
      animation: wt-spin 0.6s linear infinite;
      margin-right: 4px;
      vertical-align: middle;
    }
    .wt-bar-action-secondary:disabled .wt-spinner {
      border-color: rgba(0,0,0,0.15);
      border-top-color: var(--text-secondary);
    }
    @keyframes wt-spin { to { transform: rotate(360deg); } }
    .wt-bar-action-secondary {
      padding: 4px 10px;
      background: var(--bg-accent);
      color: var(--text-secondary);
      border-radius: var(--radius-sm);
      font-size: 11px;
      font-weight: 500;
      border: 1px solid var(--border);
      transition: background 0.12s, color 0.12s;
      flex-shrink: 0;
    }
    .wt-bar-action-secondary:hover { background: var(--bg-hover); color: var(--text); }
    .wt-bar-create-pr {
      display: flex;
      align-items: center;
      gap: 5px;
      padding: 4px 12px;
      background: var(--green);
      color: #fff;
      border-radius: var(--radius-sm);
      font-size: 11px;
      font-weight: 600;
      transition: filter 0.12s;
      flex-shrink: 0;
    }
    .wt-bar-create-pr:hover:not(:disabled) { filter: brightness(1.15); }
    .wt-bar-create-pr:disabled { opacity: 0.6; cursor: default; }

    /* ── Git migration banner ── */
    .git-migration-bar {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px 16px;
      background: rgba(240, 184, 64, 0.06);
      border-top: 1px solid rgba(240, 184, 64, 0.15);
      flex-shrink: 0;
      font-size: 11px;
      order: 99;
      animation: fade-slide-up 0.2s ease both;
    }
    .gm-icon { color: var(--amber); flex-shrink: 0; }
    .gm-text { color: var(--text-secondary); flex: 1; }
    .gm-init-btn {
      padding: 4px 12px;
      background: var(--amber);
      color: #000;
      font-weight: 600;
      font-size: 11px;
      border-radius: var(--radius-sm);
      transition: filter 0.12s;
      flex-shrink: 0;
    }
    .gm-init-btn:hover:not(:disabled) { filter: brightness(1.1); }
    .gm-init-btn:disabled { opacity: 0.6; cursor: default; }
    .wt-confirm-group {
      display: flex;
      align-items: center;
      gap: 6px;
      animation: fade-in 0.12s ease;
    }
    .wt-confirm-label {
      font-size: 11px;
      color: var(--amber);
      font-weight: 600;
      white-space: nowrap;
    }
    .wt-confirm-yes {
      padding: 4px 12px;
      background: var(--red);
      color: white;
      border-radius: var(--radius-sm);
      font-size: 11px;
      font-weight: 600;
      transition: filter 0.12s;
    }
    .wt-confirm-yes:hover { filter: brightness(1.15); }
    .wt-confirm-no {
      padding: 4px 10px;
      color: var(--text-tertiary);
      font-size: 11px;
      font-weight: 500;
      border-radius: var(--radius-sm);
      transition: background 0.12s, color 0.12s;
    }
    .wt-confirm-no:hover { background: var(--bg-accent); color: var(--text-secondary); }
    .wt-create-btn {
      padding: 5px 12px;
      background: var(--bg-accent);
      border: 1px solid var(--border);
      color: var(--text-secondary);
      border-radius: var(--radius-sm);
      font-size: 12px;
      transition: background 0.12s;
    }
    .wt-create-btn:hover { background: var(--bg-muted); }

    /* ── prefers-reduced-motion ── */
    @media (prefers-reduced-motion: reduce) {
      .msg, .msg-user, .msg-assistant, .msg-system { animation: none !important; }
      .msg-body p, .msg-body li, .msg-body pre { animation: none !important; }
      .tc, .tc-stack { animation: none !important; }
      .approval-card { animation: approval-pulse 2s ease-in-out infinite !important; }
      .typing-shimmer-line { animation: none; background: rgba(107, 124, 255, 0.15); }
    }
    `}</style>
    </>
  );
}

function renderBlock(block: ContentBlock, isLastAndStreaming: boolean) {
  switch (block.type) {
    case "text":
      return block.content.trim() ? <Markdown content={block.content} /> : null;
    case "tool_use":
      return <ToolUseCard block={block} />;
    case "thinking":
      return <ThinkingBlock content={block.content} streaming={isLastAndStreaming && block.type === "thinking"} />;
    default:
      return null;
  }
}

/**
 * Groups consecutive tool_use blocks with the same tool_name into runs.
 * Returns an array of { type, blocks, index } where type is "stack" (2+ same-name)
 * or "single" (standalone block).
 */
type BlockGroup =
  | { type: "stack"; toolName: string; blocks: ContentBlock[]; index: number }
  | { type: "single"; block: ContentBlock; index: number };

function groupBlocks(blocks: ContentBlock[]): BlockGroup[] {
  const groups: BlockGroup[] = [];
  let i = 0;
  while (i < blocks.length) {
    const block = blocks[i];
    if (block.type === "tool_use" && block.tool_name) {
      // Collect consecutive same-name tool_use blocks
      const run: ContentBlock[] = [block];
      let j = i + 1;
      while (j < blocks.length && blocks[j].type === "tool_use" && blocks[j].tool_name === block.tool_name) {
        run.push(blocks[j]);
        j++;
      }
      if (run.length >= 2) {
        groups.push({ type: "stack", toolName: block.tool_name, blocks: run, index: i });
      } else {
        groups.push({ type: "single", block, index: i });
      }
      i = j;
    } else {
      groups.push({ type: "single", block, index: i });
      i++;
    }
  }
  return groups;
}

function MessageBubble(props: { msg: any; isGenerating: boolean; isLast: boolean; isLastUser: boolean; threadId: string }) {
  const [copied, setCopied] = createSignal(false);
  const [detailsHidden, setDetailsHidden] = createSignal(false);
  const isAssistant = () => props.msg.role === "assistant";
  const meta = () => props.msg.meta;
  const hasBlocks = () => props.msg.blocks && props.msg.blocks.length > 0;
  const isStreaming = () => isAssistant() && !props.msg.id.startsWith("done-");
  const isDone = () => isAssistant() && props.msg.id.startsWith("done-");

  // Check if there are non-text blocks (tool use or thinking) worth toggling
  const hasDetails = () => {
    if (!props.msg.blocks) return false;
    return (props.msg.blocks as ContentBlock[]).some((b) => b.type !== "text");
  };

  // Get only text blocks for collapsed view
  const textBlocks = () => {
    if (!props.msg.blocks) return [];
    return (props.msg.blocks as ContentBlock[]).filter((b) => b.type === "text" && b.content.trim());
  };

  // Count details for the toggle label
  const detailCounts = () => {
    if (!props.msg.blocks) return { tools: 0, thinking: 0 };
    const blocks = props.msg.blocks as ContentBlock[];
    return {
      tools: blocks.filter((b) => b.type === "tool_use").length,
      thinking: blocks.filter((b) => b.type === "thinking").length,
    };
  };

  async function copyContent() {
    await navigator.clipboard.writeText(props.msg.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  function formatDuration(ms: number) {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  }

  function formatTokens(n: number) {
    if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
    return String(n);
  }

  function formatModel(model: string): string {
    // Shorten long model IDs to friendly names
    if (model.includes("opus")) return "Opus";
    if (model.includes("sonnet")) return "Sonnet";
    if (model.includes("haiku")) return "Haiku";
    return model;
  }

  return (
    <div class={`msg msg-${props.msg.role}`}>
      <Show when={props.msg.role === "system"}>
        <div
          class="msg-system-pill"
          classList={{
            [`msg-system-pill--${props.msg.system_kind || "info"}`]: true,
          }}
        >
          <For each={linkifyParts(props.msg.content)}>
            {(part) => part.type === "link"
              ? <span
                  class="msg-system-link"
                  role="link"
                  onClick={() => { ipc.openExternalUrl(part.value).catch(() => {}); }}
                >{part.value}</span>
              : <span>{part.value}</span>
            }
          </For>
        </div>
      </Show>
      <Show when={props.msg.role === "user"}>
        <div class="msg-user-bubble">{props.msg.content}</div>
        <div class="msg-user-actions">
          <button
            class="msg-action-btn"
            title="Edit & resend"
            onClick={() => appStore.editAndResend(props.threadId, props.msg.id, props.msg.content)}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7" />
              <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z" />
            </svg>
          </button>
          <Show when={props.isLastUser && !props.isGenerating}>
            <button
              class="msg-action-btn"
              title="Retry"
              onClick={() => appStore.retryLastMessage(props.threadId)}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="23 4 23 10 17 10" />
                <path d="M20.49 15a9 9 0 11-2.12-9.36L23 10" />
              </svg>
            </button>
          </Show>
        </div>
      </Show>
      <Show when={isAssistant()}>
        <div class="msg-assistant">
          {/* Toggle for hiding details on completed messages */}
          <Show when={isDone() && hasDetails()}>
            <button class="msg-details-toggle" onClick={() => setDetailsHidden(!detailsHidden())}>
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <Show when={detailsHidden()} fallback={<><polyline points="6 9 12 15 18 9" /></>}>
                  <polyline points="9 18 15 12 9 6" />
                </Show>
              </svg>
              <Show when={detailsHidden()} fallback="Hide details">
                {(() => {
                  const c = detailCounts();
                  const parts: string[] = [];
                  if (c.tools > 0) parts.push(`${c.tools} tool${c.tools > 1 ? "s" : ""}`);
                  if (c.thinking > 0) parts.push("thinking");
                  return `Show ${parts.join(" + ")}`;
                })()}
              </Show>
            </button>
          </Show>

          {/* Message content */}
          <div class="msg-body">
            <Show when={hasBlocks()} fallback={<Markdown content={props.msg.content} />}>
              <Show when={detailsHidden()}>
                {/* Collapsed: only text blocks */}
                <For each={textBlocks()}>
                  {(block) => <Markdown content={block.content} />}
                </For>
                <Show when={textBlocks().length === 0}>
                  <span class="msg-no-text">Response contained only tool use</span>
                </Show>
              </Show>
              <Show when={!detailsHidden()}>
                {/* Full: all blocks — stack consecutive same-name tool calls */}
                <For each={groupBlocks(props.msg.blocks as ContentBlock[])}>
                  {(group) => {
                    if (group.type === "stack") {
                      return <ToolUseStack blocks={group.blocks} />;
                    }
                    const block = group.block;
                    const isLastStreaming = isStreaming() && group.index === (props.msg.blocks as ContentBlock[]).length - 1;
                    return renderBlock(block, isLastStreaming);
                  }}
                </For>
              </Show>
            </Show>
          </div>

          {/* Footer: meta + copy */}
          <div class="msg-footer">
            <Show when={meta()}>
              <div class="msg-meta">
                <Show when={meta()!.model && meta()!.model !== "unknown"}>
                  <span class="msg-meta-tag">{formatModel(meta()!.model!)}</span>
                </Show>
                <Show when={meta()!.inputTokens != null || meta()!.outputTokens != null}>
                  <span class="msg-meta-tag">{formatTokens(meta()!.inputTokens || 0)} in / {formatTokens(meta()!.outputTokens || 0)} out</span>
                </Show>
                <Show when={meta()!.durationMs}>
                  <span class="msg-meta-tag">{formatDuration(meta()!.durationMs!)}</span>
                </Show>
                <Show when={meta()!.costUsd != null && meta()!.costUsd! > 0}>
                  <span class="msg-meta-tag">${meta()!.costUsd!.toFixed(4)}</span>
                </Show>
              </div>
            </Show>
            <Show when={props.msg.content}>
              <button class="msg-copy" onClick={copyContent} title={copied() ? "Copied!" : "Copy"}>
                <Show when={copied()} fallback={
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/>
                  </svg>
                }>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="20 6 9 17 4 12"/>
                  </svg>
                </Show>
              </button>
            </Show>
            <Show when={isDone() && !props.isGenerating}>
              <button
                class="msg-action-btn"
                title="Regenerate response"
                onClick={() => appStore.regenerateResponse(props.threadId, props.msg.id)}
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="23 4 23 10 17 10" />
                  <path d="M20.49 15a9 9 0 11-2.12-9.36L23 10" />
                </svg>
              </button>
            </Show>
          </div>
        </div>
      </Show>
    </div>
  );
}
