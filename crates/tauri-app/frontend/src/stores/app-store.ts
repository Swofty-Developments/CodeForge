/**
 * THE single global store (createRoot(createStore) singleton, CodeForge-style).
 * The store SHAPE is the frozen contract; action bodies are minimal until the
 * backend commands are implemented.
 */

import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";
import * as ipc from "../ipc";
import type {
  ActiveView,
  AgentEventPayload,
  DaemonStatus,
  DiffByFeature,
  Feature,
  IndexProgress,
  RepoState,
  SessionUi,
  TimelineEvent,
} from "../types";
import { PLACEHOLDER_FEATURES } from "./placeholder-data";

export interface AppStore {
  repo: RepoState | null;
  features: Feature[];
  /** True while `features` holds scaffold placeholder data. */
  featuresArePlaceholder: boolean;
  selectedFeature: string | null;
  activeView: ActiveView;
  timeline: TimelineEvent[];
  diff: DiffByFeature | null;
  sessions: SessionUi[];
  activeSessionId: string | null;
  indexProgress: IndexProgress | null;
  daemon: DaemonStatus | null;
  paletteOpen: boolean;
  sidebarWidth: number;
  sessionPaneOpen: boolean;
  sessionPaneWidth: number;
  lastError: string | null;
}

function createAppStore() {
  const [store, setStore] = createStore<AppStore>({
    repo: null,
    features: PLACEHOLDER_FEATURES,
    featuresArePlaceholder: true,
    selectedFeature: null,
    activeView: "welcome",
    timeline: [],
    diff: null,
    sessions: [],
    activeSessionId: null,
    indexProgress: null,
    daemon: null,
    paletteOpen: false,
    sidebarWidth: 260,
    sessionPaneOpen: true,
    sessionPaneWidth: 380,
    lastError: null,
  });

  // ── Repo lifecycle ────────────────────────────────────────────────────────

  async function openRepo(path: string): Promise<void> {
    try {
      const repo = await ipc.openRepo(path);
      setStore("repo", repo);
      setStore("lastError", null);
      await Promise.all([refreshFeatures(), refreshTimeline(), refreshDaemon()]);
      setStore("activeView", store.features.length > 0 ? "feature" : "timeline");
    } catch (e) {
      setStore("lastError", String(e));
    }
  }

  async function closeRepo(): Promise<void> {
    const repo = store.repo;
    if (!repo) return;
    try {
      await ipc.closeRepo(repo.path);
    } catch (e) {
      setStore("lastError", String(e));
    }
    setStore({
      repo: null,
      features: PLACEHOLDER_FEATURES,
      featuresArePlaceholder: true,
      selectedFeature: null,
      activeView: "welcome",
      timeline: [],
      diff: null,
      daemon: null,
      indexProgress: null,
    });
  }

  async function reindex(force: boolean): Promise<void> {
    if (!store.repo) return;
    try {
      await ipc.reindexRepo(store.repo.path, force);
    } catch (e) {
      setStore("lastError", String(e));
    }
  }

  // ── Data refresh ──────────────────────────────────────────────────────────

  async function refreshFeatures(): Promise<void> {
    if (!store.repo) return;
    try {
      const features = await ipc.getFeatures(store.repo.path);
      setStore({ features, featuresArePlaceholder: false });
    } catch (e) {
      setStore("lastError", String(e));
    }
  }

  async function refreshTimeline(): Promise<void> {
    if (!store.repo) return;
    try {
      setStore("timeline", await ipc.getTimeline(store.repo.path, { limit: 200 }));
    } catch (e) {
      setStore("lastError", String(e));
    }
  }

  async function refreshDiff(): Promise<void> {
    if (!store.repo) return;
    try {
      setStore("diff", await ipc.getDiffByFeature(store.repo.path));
    } catch (e) {
      setStore("lastError", String(e));
    }
  }

  async function refreshDaemon(): Promise<void> {
    if (!store.repo) return;
    try {
      setStore("daemon", await ipc.daemonStatus(store.repo.path));
    } catch {
      setStore("daemon", { running: false, port: null });
    }
  }

  // ── Navigation ────────────────────────────────────────────────────────────

  function selectFeature(slug: string | null): void {
    setStore("selectedFeature", slug);
    if (slug) setStore("activeView", "feature");
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

  function toggleSessionPane(): void {
    setStore("sessionPaneOpen", !store.sessionPaneOpen);
  }

  // ── Sessions ──────────────────────────────────────────────────────────────

  async function startSession(): Promise<void> {
    if (!store.repo) return;
    try {
      const info = await ipc.startSession({ repoPath: store.repo.path });
      setStore("sessions", store.sessions.length, {
        info,
        runState: "starting",
        blocks: [],
        pendingApproval: null,
        slashCommands: [],
      });
      setStore("activeSessionId", info.id);
    } catch (e) {
      setStore("lastError", String(e));
    }
  }

  async function sendSessionInput(text: string): Promise<void> {
    const id = store.activeSessionId;
    if (!id) return;
    try {
      await ipc.sendSessionInput(id, text);
    } catch (e) {
      setStore("lastError", String(e));
    }
  }

  async function approveRequest(sessionId: string, requestId: string, approve: boolean): Promise<void> {
    try {
      await ipc.approveSession(sessionId, requestId, approve);
    } catch (e) {
      setStore("lastError", String(e));
    }
  }

  async function stopSession(id: string): Promise<void> {
    try {
      await ipc.stopSession(id);
    } catch (e) {
      setStore("lastError", String(e));
    }
    setStore("sessions", (s) => s.filter((x) => x.info.id !== id));
    if (store.activeSessionId === id) setStore("activeSessionId", null);
  }

  // ── Event reducers (single global listeners, registered in main.tsx) ─────

  function handleAgentEvent(payload: AgentEventPayload): void {
    const idx = store.sessions.findIndex((s) => s.info.id === payload.sessionId);
    if (idx < 0) return;
    // Minimal reducer — the full ContentBlock semantics (tool cards, thinking,
    // backwards tool_id correlation) are specified in docs/ARCHITECTURE.md.
    switch (payload.eventType) {
      case "turn_started":
        setStore("sessions", idx, "runState", "generating");
        break;
      case "turn_completed":
      case "turn_aborted":
        setStore("sessions", idx, "runState", "ready");
        break;
      case "session_ready":
        if (store.sessions[idx].runState !== "generating") {
          setStore("sessions", idx, "runState", "ready");
        }
        if (payload.model) setStore("sessions", idx, "info", "model", payload.model);
        break;
      case "content_delta": {
        const blocks = store.sessions[idx].blocks;
        const last = blocks[blocks.length - 1];
        if (last && last.type === "text") {
          setStore("sessions", idx, "blocks", blocks.length - 1, "content", (c) => c + (payload.text ?? ""));
        } else {
          setStore("sessions", idx, "blocks", blocks.length, { type: "text", content: payload.text ?? "" });
        }
        break;
      }
      case "approval_required":
        setStore("sessions", idx, "pendingApproval", {
          requestId: payload.requestId ?? "",
          description: payload.description ?? "",
        });
        break;
      case "slash_commands":
        setStore("sessions", idx, "slashCommands", payload.commands ?? []);
        break;
      case "session_error":
        setStore("sessions", idx, "runState", "error");
        break;
      default:
        break;
    }
  }

  function handleTimelineEvent(event: TimelineEvent): void {
    setStore("timeline", (t) => [event, ...t]);
  }

  function handleIndexProgress(progress: IndexProgress): void {
    setStore("indexProgress", progress.done >= progress.total && progress.total > 0 ? null : progress);
  }

  function handleRepoChanged(repo: RepoState): void {
    setStore("repo", repo);
    void refreshFeatures();
  }

  return {
    store,
    setStore,
    openRepo,
    closeRepo,
    reindex,
    refreshFeatures,
    refreshTimeline,
    refreshDiff,
    refreshDaemon,
    selectFeature,
    setActiveView,
    setPaletteOpen,
    setSidebarWidth,
    toggleSessionPane,
    startSession,
    sendSessionInput,
    approveRequest,
    stopSession,
    handleAgentEvent,
    handleTimelineEvent,
    handleIndexProgress,
    handleRepoChanged,
  };
}

export const appStore = createRoot(createAppStore);
