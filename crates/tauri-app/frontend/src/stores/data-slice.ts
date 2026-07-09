/**
 * Data slice — per-context data loads (features / timeline / diff / daemon) and
 * the feature mutations (reindex, pin, edit, color, select). Everything here
 * reads the derived `repo` getter, so it always acts on the ACTIVE context.
 */

import { type SetStoreFunction } from "solid-js/store";
import * as ipc from "../ipc";
import type { AppStore } from "./app-store";
import type { FeaturePatch } from "../types";

export function createDataSlice(
  store: AppStore,
  setStore: SetStoreFunction<AppStore>,
  pushError: (message: string) => void,
) {
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

  async function reindex(force = false): Promise<void> {
    if (!store.repo) return;
    try {
      await ipc.reindexRepo(store.repo.path, force);
    } catch (e) {
      pushError(String(e));
    }
  }

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

  /** Set (or clear, when color is undefined) a feature's graph color. Optimistic. */
  async function setFeatureColor(slug: string, color?: string): Promise<void> {
    if (!store.repo) return;
    const i = store.features.findIndex((f) => f.slug === slug);
    const prev = i >= 0 ? store.features[i].color : undefined;
    if (i >= 0) setStore("features", i, "color", color);
    try {
      const updated = await ipc.setFeatureColor(store.repo.path, slug, color);
      if (i >= 0) setStore("features", i, updated);
    } catch (e) {
      if (i >= 0) setStore("features", i, "color", prev);
      pushError(String(e));
    }
  }

  /** Select a feature and load its detail data. Does NOT change the active view. */
  function selectFeature(slug: string | null): void {
    setStore({ selectedFeature: slug, selectedFeatureTimeline: [], selectedFeatureDoc: null });
    if (!slug) return;
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

  /** Select a feature AND open its detail (sidebar / palette / graph "open"). */
  function openFeatureDetail(slug: string): void {
    selectFeature(slug);
    setStore("activeView", "feature");
  }

  return {
    refreshFeatures,
    refreshTimeline,
    refreshDiff,
    refreshDaemon,
    reindex,
    pinFeature,
    updateFeature,
    setFeatureColor,
    selectFeature,
    openFeatureDetail,
  };
}
