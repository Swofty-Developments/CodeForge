/* Sidebar-local collapse state for the feature tree (FZ-1). Keyed by a group's
 * full slash path; default is expanded (absent = expanded, true = collapsed).
 * Lives here — not in app-store — so the tree owns its own UI bit. */

import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";

const [collapsed, setCollapsed] = createRoot(() => createStore<Record<string, boolean>>({}));

/** Truthy = the group at this path is collapsed. */
export const collapsedGroups = collapsed;

export function toggleGroup(path: string): void {
  setCollapsed(path, (v) => !v);
}
