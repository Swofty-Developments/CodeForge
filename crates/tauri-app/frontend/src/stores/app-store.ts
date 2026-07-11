/**
 * THE single global store (createRoot(createStore) singleton, CodeForge-style).
 * The store SHAPE is the frozen contract. Actions live in slices:
 *   session-slice.ts  — session lifecycle + agent-event reducer
 *   context-slice.ts  — the multi-context (base + worktrees) model
 *   data-slice.ts     — per-context data loads + feature mutations
 *
 * `repo` is a DERIVED getter = the active context's RepoState, so every existing
 * view that reads store.repo keeps working while the app tracks N open contexts.
 */

import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";
import * as ipc from "../ipc";
import type {
  ActiveView,
  DaemonState,
  DiffByFeature,
  ErrorToast,
  Feature,
  IndexProgress,
  IndexStatus,
  MergeResult,
  RepoContext,
  RepoState,
  SessionUi,
  TerminalTab,
  TimelineEvent,
  Worktree,
} from "../types";
import { createSessionSlice } from "./session-slice";
import { createContextSlice } from "./context-slice";
import { createDataSlice } from "./data-slice";
import { createTerminalSlice } from "./terminal-slice";
import { normPath, samePath } from "./path";

const TIMELINE_CAP = 1000;
const TOAST_DISMISS_MS = 5000;

export interface AppStore {
  /** Derived: the active context's RepoState (or null). Never written directly. */
  readonly repo: RepoState | null;
  /** All open contexts (base + worktrees), keyed by state.path. */
  contexts: RepoContext[];
  activeContextPath: string | null;
  /** list_worktrees for the active repo family (base first); powers the tab strip. */
  worktrees: Worktree[];
  worktreesLoading: boolean;
  /** Last merge outcome, surfaced honestly (clean success OR conflict panel). */
  mergeResult: MergeResult | null;
  /** Worktree switcher popover — tab-strip "+" and TitleBar "Worktrees…". */
  worktreeSwitcherOpen: boolean;
  features: Feature[];
  /** Legacy scaffold flag — always false now that the backend is wired. */
  featuresArePlaceholder: boolean;
  selectedFeature: string | null;
  /** Recent timeline slice for the selected feature (FeatureDetail). */
  selectedFeatureTimeline: TimelineEvent[];
  /** Living-doc markdown (.codeforge/docs/<slug>.md) for the selected feature. */
  selectedFeatureDoc: string | null;
  activeView: ActiveView;
  timeline: TimelineEvent[];
  /** Per-feature live event counters (sidebar activity badges). */
  featureActivity: Record<string, number>;
  diff: DiffByFeature | null;
  sessions: SessionUi[];
  activeSessionId: string | null;
  indexProgress: IndexProgress | null;
  daemon: DaemonState | null;
  paletteOpen: boolean;
  sidebarWidth: number;
  sessionPaneOpen: boolean;
  sessionPaneWidth: number;
  sessionPaneFullscreen: boolean;
  /** Text waiting to be inserted into the session composer ("Ask Claude" flow). */
  composerPrefill: string | null;
  /** Bottom terminal panel (FZ-3) — worktree-scoped PTY tabs. */
  terminals: TerminalTab[];
  activeTerminalId: string | null;
  terminalPanelOpen: boolean;
  terminalPanelHeight: number;
  /** Pending re-index prompt (FZ-2), keyed to the repo whose index went stale. */
  staleModal: { repoPath: string; status: IndexStatus } | null;
  /** Latest index:status per repo path — drives the status-bar freshness dot. */
  indexStatusByPath: Record<string, IndexStatus>;
  lastError: string | null;
  toasts: ErrorToast[];
}

function createAppStore() {
  const [store, setStore] = createStore<AppStore>({
    get repo(): RepoState | null {
      const active = this.activeContextPath;
      if (!active) return null;
      const ctx = this.contexts.find((c) => samePath(c.state.path, active));
      return ctx ? ctx.state : null;
    },
    contexts: [],
    activeContextPath: null,
    worktrees: [],
    worktreesLoading: false,
    mergeResult: null,
    worktreeSwitcherOpen: false,
    features: [],
    featuresArePlaceholder: false,
    selectedFeature: null,
    selectedFeatureTimeline: [],
    selectedFeatureDoc: null,
    activeView: "welcome",
    timeline: [],
    featureActivity: {},
    diff: null,
    sessions: [],
    activeSessionId: null,
    indexProgress: null,
    daemon: null,
    paletteOpen: false,
    sidebarWidth: 260,
    sessionPaneOpen: true,
    sessionPaneWidth: 380,
    sessionPaneFullscreen: false,
    composerPrefill: null,
    terminals: [],
    activeTerminalId: null,
    terminalPanelOpen: false,
    terminalPanelHeight: 240,
    staleModal: null,
    indexStatusByPath: {},
    lastError: null,
    toasts: [],
  });

  // ── Toast surface ─────────────────────────────────────────────────────────

  let toastSeq = 0;

  function pushToast(message: string, kind: "error" | "success" = "error"): void {
    const id = ++toastSeq;
    if (kind === "error") setStore("lastError", message);
    setStore("toasts", (t) => [...t, { id, message, kind }]);
    setTimeout(() => dismissToast(id), TOAST_DISMISS_MS);
  }
  const pushError = (message: string): void => pushToast(message, "error");
  const pushSuccess = (message: string): void => pushToast(message, "success");

  function dismissToast(id: number): void {
    setStore("toasts", (t) => t.filter((x) => x.id !== id));
  }

  // ── Slices ────────────────────────────────────────────────────────────────

  const dataSlice = createDataSlice(store, setStore, pushError);
  const contextSlice = createContextSlice(store, setStore, pushError, {
    refreshFeatures: dataSlice.refreshFeatures,
    refreshTimeline: dataSlice.refreshTimeline,
    refreshDiff: dataSlice.refreshDiff,
    refreshDaemon: dataSlice.refreshDaemon,
  });
  const sessionSlice = createSessionSlice(store, setStore, pushError);
  const terminalSlice = createTerminalSlice(store, setStore, pushError);

  // ── Index staleness (FZ-2) — re-index prompt + per-repo suppression ────────

  const staleKey = (p: string): string => `ff:stale-dismissed:${normPath(p)}`;
  const isStaleSuppressed = (p: string): boolean => localStorage.getItem(staleKey(p)) === "1";

  /** Track which repo+state episodes have had their modal shown this session.
   *  Cleared when the state transitions away (e.g. stale→fresh) or when the user
   *  acts on the modal (reindex). This separates "status consumption" (always
   *  update the dot) from "modal presentation" (show once per episode). */
  const shownStaleModals = new Set<string>();

  /** index:status event consumer — the backend's 10s poller emits on ANY status
   *  change (including growing changedFiles). Two concerns, cleanly separated:
   *
   *  1. **Status tracking** (status bar dot) — always consume, keep current.
   *  2. **Modal triggering** (re-index prompt) — show once per state episode.
   *
   *  Modal pops when: (a) state is stale/outdated, (b) not suppressed, (c) this
   *  state episode hasn't been shown yet. When the state transitions back to
   *  fresh/never, the episode ends and the flag is cleared. When the user acts
   *  on the modal (reindex), the flag is cleared so a future stale→fresh→stale
   *  cycle will re-prompt. */
  function handleIndexStatus(repoPath: string, status: IndexStatus): void {
    setStore("indexStatusByPath", repoPath, status);

    const episodeKey = `${repoPath}:${status.state}`;

    // If the state transitioned to a non-actionable state, clear the shown flag
    // so the next stale/outdated episode will prompt.
    if (status.state !== "stale" && status.state !== "outdated") {
      shownStaleModals.delete(episodeKey);
      return;
    }

    // Don't re-show if the user suppressed this repo or we've already shown this episode.
    if (isStaleSuppressed(repoPath)) return;
    if (shownStaleModals.has(episodeKey)) return;

    // Show the modal and mark this episode as shown.
    shownStaleModals.add(episodeKey);
    setStore("staleModal", { repoPath, status });
  }

  const dismissStaleModal = (): void => setStore("staleModal", null);

  /** "Don't ask again" for this repo — persist and close. */
  function suppressStale(repoPath: string): void {
    localStorage.setItem(staleKey(repoPath), "1");
    if (store.staleModal && samePath(store.staleModal.repoPath, repoPath)) setStore("staleModal", null);
  }

  /** Re-index the modal's repo (force) and close. Targets that repoPath, not
   *  necessarily the active context — they can differ per-worktree. Clear the
   *  shown flag so a future stale episode will re-prompt. */
  async function reindexStale(repoPath: string): Promise<void> {
    const current = store.staleModal;
    if (current) {
      shownStaleModals.delete(`${repoPath}:${current.status.state}`);
    }
    setStore("staleModal", null);
    try {
      await ipc.reindexRepo(repoPath, true);
    } catch (e) {
      pushError(String(e));
    }
  }

  // ── Navigation / layout ───────────────────────────────────────────────────

  function setActiveView(view: ActiveView): void {
    setStore("activeView", view);
    if (view === "diff") void dataSlice.refreshDiff();
    if (view === "timeline") void dataSlice.refreshTimeline();
  }

  function setPaletteOpen(open: boolean): void {
    setStore("paletteOpen", open);
  }

  function setWorktreeSwitcherOpen(open: boolean): void {
    setStore("worktreeSwitcherOpen", open);
  }

  function setSidebarWidth(px: number): void {
    setStore("sidebarWidth", Math.min(500, Math.max(180, px)));
  }

  function setSessionPaneWidth(px: number): void {
    setStore("sessionPaneWidth", Math.min(640, Math.max(280, px)));
  }

  function toggleSessionPane(): void {
    setStore("sessionPaneOpen", !store.sessionPaneOpen);
  }

  function toggleSessionPaneFullscreen(): void {
    setStore("sessionPaneFullscreen", !store.sessionPaneFullscreen);
  }

  // ── Event reducers (single global listeners, registered via initListeners) ─

  /** FZ-5: append a LIVE timeline event only when it belongs to the active
   *  context. Background contexts reload their timeline on switch, so appending
   *  their live events here would leak across worktrees. */
  function handleTimelineEvent(repoPath: string, event: TimelineEvent): void {
    if (!store.activeContextPath || !samePath(repoPath, store.activeContextPath)) return;
    setStore("timeline", (t) => [event, ...t].slice(0, TIMELINE_CAP));
    for (const slug of event.featureSlugs) {
      setStore("featureActivity", slug, (n) => (n ?? 0) + 1);
    }
    if (store.selectedFeature && event.featureSlugs.includes(store.selectedFeature)) {
      setStore("selectedFeatureTimeline", (t) => [event, ...t].slice(0, 100));
    }
    // Living-doc hot reload: an auto-refresh landed for the OPEN feature — pull
    // the fresh doc so FeatureDetail updates in place. Only the "updated"
    // outcome refetches; a failed refresh stays visible on the timeline.
    const repo = store.repo;
    if (
      event.kind === "doc_updated" &&
      event.payload.outcome === "updated" &&
      event.payload.slug === store.selectedFeature &&
      repo
    ) {
      const slug = event.payload.slug;
      void ipc
        .getFeatureDoc(repo.path, slug)
        .then((doc) => {
          // Stale-response guard: apply only if this feature is still open.
          if (store.selectedFeature === slug) setStore("selectedFeatureDoc", doc);
        })
        .catch((e) => pushError(String(e)));
    }
  }

  function handleIndexProgress(progress: IndexProgress): void {
    const done = progress.total > 0 && progress.done >= progress.total;
    setStore("indexProgress", done ? null : progress);
    if (done) {
      void dataSlice.refreshFeatures();
      void dataSlice.refreshTimeline();
    }
  }

  function handleRepoChanged(repo: RepoState): void {
    const i = store.contexts.findIndex((c) => samePath(c.state.path, repo.path));
    if (i >= 0) setStore("contexts", i, "state", repo);
    if (store.activeContextPath && samePath(store.activeContextPath, repo.path)) {
      void dataSlice.refreshFeatures();
    }
  }

  // ── Listener registration (once per app run) ──────────────────────────────

  let listenersRegistered = false;

  async function initListeners(): Promise<void> {
    if (listenersRegistered) return;
    listenersRegistered = true;
    try {
      await Promise.all([
        ipc.listenAgentEvent(sessionSlice.handleAgentEvent),
        ipc.listenTimelineEvent(handleTimelineEvent),
        ipc.listenIndexProgress(handleIndexProgress),
        ipc.listenIndexStatus(handleIndexStatus),
        ipc.listenRepoChanged(handleRepoChanged),
      ]);
    } catch {
      // Not running inside Tauri (plain `vite dev` in a browser) — fine for UI work.
    }
  }

  return {
    store,
    setStore,
    ...dataSlice,
    ...contextSlice,
    ...sessionSlice,
    ...terminalSlice,
    handleIndexStatus,
    dismissStaleModal,
    suppressStale,
    reindexStale,
    setActiveView,
    setPaletteOpen,
    setWorktreeSwitcherOpen,
    setSidebarWidth,
    setSessionPaneWidth,
    toggleSessionPane,
    toggleSessionPaneFullscreen,
    pushError,
    pushSuccess,
    dismissToast,
    handleTimelineEvent,
    handleIndexProgress,
    handleRepoChanged,
    initListeners,
  };
}

export const appStore = createRoot(createAppStore);
