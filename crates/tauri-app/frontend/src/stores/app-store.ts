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
  MergeResult,
  RepoContext,
  RepoState,
  SessionUi,
  TimelineEvent,
  Worktree,
} from "../types";
import { createSessionSlice } from "./session-slice";
import { createContextSlice } from "./context-slice";
import { createDataSlice } from "./data-slice";
import { samePath } from "./path";

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
  worktreePromptOpen: boolean;
  features: Feature[];
  /** Legacy scaffold flag — always false now that the backend is wired. */
  featuresArePlaceholder: boolean;
  selectedFeature: string | null;
  /** Recent timeline slice for the selected feature (FeatureDetail). */
  selectedFeatureTimeline: TimelineEvent[];
  /** Living-doc markdown (.featureforge/docs/<slug>.md) for the selected feature. */
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
  /** Text waiting to be inserted into the session composer ("Ask Claude" flow). */
  composerPrefill: string | null;
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
    worktreePromptOpen: false,
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
    composerPrefill: null,
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

  // ── Navigation / layout ───────────────────────────────────────────────────

  function setActiveView(view: ActiveView): void {
    setStore("activeView", view);
    if (view === "diff") void dataSlice.refreshDiff();
    if (view === "timeline") void dataSlice.refreshTimeline();
  }

  function setPaletteOpen(open: boolean): void {
    setStore("paletteOpen", open);
  }

  function setWorktreePromptOpen(open: boolean): void {
    setStore("worktreePromptOpen", open);
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

  // ── Event reducers (single global listeners, registered via initListeners) ─

  function handleTimelineEvent(event: TimelineEvent): void {
    setStore("timeline", (t) => [event, ...t].slice(0, TIMELINE_CAP));
    for (const slug of event.featureSlugs) {
      setStore("featureActivity", slug, (n) => (n ?? 0) + 1);
    }
    if (store.selectedFeature && event.featureSlugs.includes(store.selectedFeature)) {
      setStore("selectedFeatureTimeline", (t) => [event, ...t].slice(0, 100));
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
    setActiveView,
    setPaletteOpen,
    setWorktreePromptOpen,
    setSidebarWidth,
    setSessionPaneWidth,
    toggleSessionPane,
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
