/**
 * THE single global store (createRoot(createStore) singleton, CodeForge-style).
 * The store SHAPE is the frozen contract; session actions + the agent-event
 * reducer live in session-slice.ts.
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
  FeaturePatch,
  IndexProgress,
  RepoState,
  SessionUi,
  TimelineEvent,
} from "../types";
import { createSessionSlice } from "./session-slice";

const TIMELINE_CAP = 1000;
const TOAST_DISMISS_MS = 5000;

export interface AppStore {
  repo: RepoState | null;
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
    repo: null,
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

  // ── Error surface ─────────────────────────────────────────────────────────

  let toastSeq = 0;

  function pushError(message: string): void {
    const id = ++toastSeq;
    setStore("lastError", message);
    setStore("toasts", (t) => [...t, { id, message }]);
    setTimeout(() => dismissToast(id), TOAST_DISMISS_MS);
  }

  function dismissToast(id: number): void {
    setStore("toasts", (t) => t.filter((x) => x.id !== id));
  }

  // ── Repo lifecycle ────────────────────────────────────────────────────────

  async function openRepo(path: string): Promise<void> {
    try {
      const repo = await ipc.openRepo(path);
      setStore({ repo, lastError: null, featureActivity: {}, selectedFeature: null, selectedFeatureTimeline: [] });
      await Promise.all([refreshFeatures(), refreshTimeline(), refreshDaemon()]);
      setStore("activeView", store.features.length > 0 ? "feature" : "timeline");
    } catch (e) {
      pushError(String(e));
    }
  }

  async function closeRepo(): Promise<void> {
    const repo = store.repo;
    if (!repo) return;
    try {
      await ipc.closeRepo(repo.path);
    } catch (e) {
      pushError(String(e));
    }
    setStore({
      repo: null,
      features: [],
      selectedFeature: null,
      selectedFeatureTimeline: [],
      activeView: "welcome",
      timeline: [],
      featureActivity: {},
      diff: null,
      daemon: null,
      indexProgress: null,
      composerPrefill: null,
    });
  }

  async function reindex(force = false): Promise<void> {
    if (!store.repo) return;
    try {
      await ipc.reindexRepo(store.repo.path, force);
    } catch (e) {
      pushError(String(e));
    }
  }

  // ── Features ──────────────────────────────────────────────────────────────

  async function pinFeature(slug: string, pinned: boolean): Promise<void> {
    if (!store.repo) return;
    const i = store.features.findIndex((f) => f.slug === slug);
    if (i >= 0) setStore("features", i, "pinned", pinned); // optimistic
    try {
      await ipc.pinFeature(store.repo.path, slug, pinned);
    } catch (e) {
      if (i >= 0) setStore("features", i, "pinned", !pinned);
      pushError(String(e));
    }
  }

  async function updateFeature(slug: string, patch: FeaturePatch): Promise<void> {
    if (!store.repo) return;
    try {
      const updated = await ipc.updateFeature(store.repo.path, slug, patch);
      const i = store.features.findIndex((f) => f.slug === slug);
      if (i >= 0) setStore("features", i, updated);
    } catch (e) {
      pushError(String(e));
    }
  }

  // ── Data refresh ──────────────────────────────────────────────────────────

  async function refreshFeatures(): Promise<void> {
    if (!store.repo) return;
    try {
      const features = await ipc.getFeatures(store.repo.path);
      setStore({ features, featuresArePlaceholder: false });
    } catch (e) {
      pushError(String(e));
    }
  }

  async function refreshTimeline(): Promise<void> {
    if (!store.repo) return;
    try {
      setStore("timeline", await ipc.getTimeline(store.repo.path, { limit: 200 }));
    } catch (e) {
      pushError(String(e));
    }
  }

  async function refreshDiff(): Promise<void> {
    if (!store.repo) return;
    try {
      setStore("diff", await ipc.getDiffByFeature(store.repo.path));
    } catch (e) {
      pushError(String(e));
    }
  }

  async function refreshDaemon(): Promise<void> {
    if (!store.repo) {
      setStore("daemon", null);
      return;
    }
    try {
      const status = await ipc.daemonStatus(store.repo.path);
      // running / offline are the two answers the backend can give; an errored
      // probe becomes `unknown` (below), never a definitive `offline`.
      setStore("daemon", status.running ? { kind: "running", port: status.port } : { kind: "offline" });
    } catch (e) {
      setStore("daemon", { kind: "unknown", error: String(e) });
    }
  }

  // ── Navigation ────────────────────────────────────────────────────────────

  function selectFeature(slug: string | null): void {
    setStore({ selectedFeature: slug, selectedFeatureTimeline: [], selectedFeatureDoc: null });
    if (!slug) return;
    setStore("activeView", "feature");
    const repo = store.repo;
    if (!repo) return;
    void (async () => {
      try {
        const [feature, events, doc] = await Promise.all([
          ipc.getFeature(repo.path, slug),
          ipc.getTimeline(repo.path, { featureSlug: slug, limit: 50 }),
          ipc.getFeatureDoc(repo.path, slug),
        ]);
        const i = store.features.findIndex((f) => f.slug === slug);
        if (i >= 0) setStore("features", i, feature);
        // Stale-response guard: only apply if this feature is still selected.
        if (store.selectedFeature === slug) {
          setStore("selectedFeatureTimeline", events);
          setStore("selectedFeatureDoc", doc);
        }
      } catch (e) {
        pushError(String(e));
      }
    })();
  }

  function setActiveView(view: ActiveView): void {
    setStore("activeView", view);
    if (view === "diff") void refreshDiff();
    if (view === "timeline") void refreshTimeline();
  }

  function setPaletteOpen(open: boolean): void {
    setStore("paletteOpen", open);
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

  // ── Sessions (see session-slice.ts) ───────────────────────────────────────

  const sessionSlice = createSessionSlice(store, setStore, pushError);

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
      void refreshFeatures();
      void refreshTimeline();
    }
  }

  function handleRepoChanged(repo: RepoState): void {
    setStore("repo", repo);
    void refreshFeatures();
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
    openRepo,
    closeRepo,
    reindex,
    pinFeature,
    updateFeature,
    refreshFeatures,
    refreshTimeline,
    refreshDiff,
    refreshDaemon,
    selectFeature,
    setActiveView,
    setPaletteOpen,
    setSidebarWidth,
    setSessionPaneWidth,
    toggleSessionPane,
    pushError,
    dismissToast,
    ...sessionSlice,
    handleTimelineEvent,
    handleIndexProgress,
    handleRepoChanged,
    initListeners,
  };
}

export const appStore = createRoot(createAppStore);
