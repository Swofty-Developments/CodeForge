/**
 * Repo-context slice — the multi-context (base + worktrees) model.
 *
 * W1: a worktree is opened as its OWN RepoRuntime via open_repo(path); each gets
 * its own daemon / feature index / timeline, keyed by its path. The store tracks
 * `contexts` (base first) + `activeContextPath`; the derived `repo` getter (in
 * app-store) returns the active context's RepoState so existing views keep working.
 *
 * Top-level features / timeline / diff always reflect the ACTIVE context and are
 * reloaded on every switch. Sessions are global but tagged with `contextPath`.
 */

import { produce, type SetStoreFunction } from "solid-js/store";
import * as ipc from "../ipc";
import type { AppStore } from "./app-store";
import type { RepoState, Worktree } from "../types";
import { samePath } from "./path";

export interface ContextDeps {
  refreshFeatures: () => Promise<void>;
  refreshTimeline: () => Promise<void>;
  refreshDiff: () => Promise<void>;
  refreshDaemon: () => Promise<void>;
}

export function createContextSlice(
  store: AppStore,
  setStore: SetStoreFunction<AppStore>,
  pushError: (message: string) => void,
  deps: ContextDeps,
) {
  /** Reload the four per-context datasets for whatever context is active. */
  function loadForActive(): Promise<unknown> {
    return Promise.all([
      deps.refreshFeatures(),
      deps.refreshTimeline(),
      deps.refreshDiff(),
      deps.refreshDaemon(),
    ]);
  }

  /** The most recent session belonging to `path`, or null. */
  function pickContextSession(path: string): string | null {
    const inCtx = store.sessions.filter((s) => samePath(s.contextPath, path));
    return inCtx.length > 0 ? inCtx[inCtx.length - 1].info.id : null;
  }

  /** Wipe the per-context view state and re-point the active session. */
  function resetContextView(path: string): void {
    setStore({
      selectedFeature: null,
      selectedFeatureTimeline: [],
      selectedFeatureDoc: null,
      diff: null,
      featureActivity: {},
      activeSessionId: pickContextSession(path),
    });
  }

  function findContext(path: string) {
    return store.contexts.find((c) => samePath(c.state.path, path));
  }

  /** The authoritative base context, or null — never a guessed substitute. */
  function baseContext() {
    return store.contexts.find((c) => c.isBase) ?? null;
  }

  /** After list_worktrees lands, refine each open context's isBase from git truth. */
  function reconcileContextBase(): void {
    const wts = store.worktrees;
    setStore(
      "contexts",
      produce((cs) => {
        for (const c of cs) {
          const m = wts.find((w) => samePath(w.path, c.state.path));
          if (m) c.isBase = m.isBase;
        }
      }),
    );
  }

  async function refreshWorktrees(): Promise<void> {
    const repo = store.repo;
    if (!repo) {
      setStore("worktrees", []);
      return;
    }
    setStore("worktreesLoading", true);
    try {
      setStore("worktrees", await ipc.listWorktrees(repo.path));
      reconcileContextBase();
    } catch (e) {
      pushError(String(e));
    } finally {
      setStore("worktreesLoading", false);
    }
  }

  /** Push a new context (or activate the existing one) and load its data. */
  async function adoptContext(repo: RepoState, isBase: boolean): Promise<void> {
    const existing = findContext(repo.path);
    if (existing) {
      setStore(
        "contexts",
        store.contexts.findIndex((c) => samePath(c.state.path, repo.path)),
        "state",
        repo,
      );
    } else {
      setStore("contexts", (cs) => [...cs, { state: repo, isBase }]);
    }
    setStore("activeContextPath", repo.path);
    resetContextView(repo.path);
    await loadForActive();
    setStore("activeView", store.features.length > 0 ? "feature" : "timeline");
    void refreshWorktrees();
  }

  /** Fresh repo open (Welcome / palette). Treated as a base context. */
  async function openRepo(path: string): Promise<void> {
    const existing = findContext(path);
    if (existing) {
      await switchContext(existing.state.path);
      return;
    }
    try {
      const repo = await ipc.openRepo(path);
      setStore("lastError", null);
      await adoptContext(repo, store.contexts.length === 0);
    } catch (e) {
      pushError(String(e));
    }
  }

  /** Open a worktree as its own context (W1) and switch to it. */
  async function openWorktreeContext(wt: Worktree): Promise<void> {
    const path = String(wt.path);
    const existing = findContext(path);
    if (existing) {
      await switchContext(existing.state.path);
      return;
    }
    try {
      const repo = await ipc.openRepo(path);
      await adoptContext(repo, wt.isBase);
    } catch (e) {
      pushError(String(e));
    }
  }

  /** Switch the active context: reload features + timeline + diff for its path
   *  and re-point sessions (W1). Cheap no-op when already active. */
  async function switchContext(path: string): Promise<void> {
    if (store.activeContextPath && samePath(store.activeContextPath, path)) return;
    const ctx = findContext(path);
    if (!ctx) {
      pushError(`no open context for ${path}`);
      return;
    }
    setStore("activeContextPath", ctx.state.path);
    resetContextView(ctx.state.path);
    if (store.activeView === "welcome") setStore("activeView", "feature");
    await loadForActive();
    void refreshWorktrees();
  }

  async function createWorktree(name: string, baseRef?: string): Promise<void> {
    const repo = store.repo;
    if (!repo) return;
    try {
      const wt = await ipc.createWorktree(repo.path, name, baseRef);
      await openWorktreeContext(wt);
      await refreshWorktrees();
    } catch (e) {
      pushError(String(e));
    }
  }

  /** Merge a worktree's branch into the base branch; surfaces MergeResult via
   *  store.mergeResult (a clean success OR an honest conflict panel — never a
   *  fabricated success). */
  async function mergeWorktree(worktreePath: string): Promise<void> {
    const base = baseContext();
    if (!base) {
      pushError("no base repository context to merge into");
      return;
    }
    try {
      const res = await ipc.mergeWorktree(base.state.path, worktreePath);
      setStore("mergeResult", res);
      await refreshWorktrees();
      if (res.merged && store.activeContextPath && samePath(store.activeContextPath, base.state.path)) {
        await loadForActive();
      }
    } catch (e) {
      pushError(String(e));
    }
  }

  /** Close a context. A worktree is removed from git (remove_worktree, confirm
   *  dirty via `force` in the caller); the base tears the whole repo down. */
  async function closeContext(path: string, force: boolean): Promise<void> {
    const ctx = findContext(path);
    if (!ctx) return;
    if (ctx.isBase) {
      await closeRepo();
      return;
    }
    const base = baseContext();
    try {
      await ipc.removeWorktree(base ? base.state.path : path, path, force);
    } catch (e) {
      pushError(String(e));
      return;
    }
    try {
      await ipc.closeRepo(path);
    } catch (e) {
      pushError(String(e));
    }
    setStore("contexts", (cs) => cs.filter((c) => !samePath(c.state.path, path)));
    setStore("sessions", (ss) => ss.filter((s) => !samePath(s.contextPath, path)));
    if (store.activeContextPath && samePath(store.activeContextPath, path)) {
      setStore("activeContextPath", null);
      const next = baseContext();
      if (next) await switchContext(next.state.path);
    }
    await refreshWorktrees();
  }

  /** Close the whole repo family — every open context's runtime — and reset. */
  async function closeRepo(): Promise<void> {
    for (const c of store.contexts) {
      try {
        await ipc.closeRepo(c.state.path);
      } catch (e) {
        pushError(String(e));
      }
    }
    setStore({
      contexts: [],
      activeContextPath: null,
      worktrees: [],
      worktreesLoading: false,
      mergeResult: null,
      features: [],
      selectedFeature: null,
      selectedFeatureTimeline: [],
      selectedFeatureDoc: null,
      activeView: "welcome",
      timeline: [],
      featureActivity: {},
      diff: null,
      daemon: null,
      indexProgress: null,
      composerPrefill: null,
      sessions: [],
      activeSessionId: null,
    });
  }

  return {
    openRepo,
    closeRepo,
    openWorktreeContext,
    switchContext,
    closeContext,
    refreshWorktrees,
    createWorktree,
    mergeWorktree,
    dismissMergeResult: () => setStore("mergeResult", null),
  };
}
