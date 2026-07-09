/**
 * Session-pane store-gap helpers — thin wrappers over appStore actions the
 * SessionPane calls by name. All session / usage / message state lives in the
 * store (the single source of truth); this module holds no parallel state and
 * registers no second `agent-event` listener.
 */

import { appStore } from "../../stores/app-store";
import type { PermissionMode } from "../../types";

/** Start a session at a chosen model + permission mode; the store builds the
 *  full SessionUi and tags it with the active context. */
export async function startSessionWithModel(model?: string, permissionMode?: PermissionMode): Promise<void> {
  await appStore.startSession(model, permissionMode);
}

export function selectSession(sessionId: string): void {
  appStore.setStore("activeSessionId", sessionId);
}

/** Stops the session; refocuses the last remaining tab, if any. */
export async function closeSession(sessionId: string): Promise<void> {
  await appStore.stopSession(sessionId);
  const { store, setStore } = appStore;
  if (store.activeSessionId === null && store.sessions.length > 0) {
    setStore("activeSessionId", store.sessions[store.sessions.length - 1].info.id);
  }
}
