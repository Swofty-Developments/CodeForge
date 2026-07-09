/**
 * Session slice — start/send/approve/stop actions + live permission-mode switch.
 * The `agent-event` reducer and its message helpers live in session-reducer.ts.
 * Sessions are tagged with the active context path (W1) and a permission mode (W5).
 */

import { produce, type SetStoreFunction } from "solid-js/store";
import * as ipc from "../ipc";
import type { PermissionMode, SessionInfo, SessionUi } from "../types";
import type { AppStore } from "./app-store";
import { createAgentEventHandler, finalizeLiveAssistant } from "./session-reducer";

function newSessionUi(info: SessionInfo, contextPath: string, permissionMode: PermissionMode): SessionUi {
  return {
    info,
    runState: "starting",
    messages: [],
    pendingApproval: null,
    slashCommands: [],
    claudeSessionId: null,
    usage: { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheWriteTokens: 0, costUsd: 0 },
    contextPath,
    permissionMode,
  };
}

export function createSessionSlice(
  store: AppStore,
  setStore: SetStoreFunction<AppStore>,
  pushError: (message: string) => void,
) {
  async function startSession(model?: string, permissionMode: PermissionMode = "default"): Promise<void> {
    if (!store.repo) return;
    const contextPath = store.activeContextPath;
    if (!contextPath) return;
    try {
      const info = await ipc.startSession({ repoPath: store.repo.path, model, permissionMode });
      setStore("sessions", store.sessions.length, newSessionUi(info, contextPath, permissionMode));
      setStore("activeSessionId", info.id);
    } catch (e) {
      pushError(String(e));
    }
  }

  /** Switch a running session's permission mode live (W5). Optimistic. */
  async function setSessionMode(sessionId: string, mode: PermissionMode): Promise<void> {
    const idx = store.sessions.findIndex((s) => s.info.id === sessionId);
    const prev = idx >= 0 ? store.sessions[idx].permissionMode : undefined;
    if (idx >= 0) setStore("sessions", idx, "permissionMode", mode);
    try {
      await ipc.setSessionMode(sessionId, mode);
    } catch (e) {
      if (idx >= 0 && prev) setStore("sessions", idx, "permissionMode", prev);
      pushError(String(e));
    }
  }

  async function sendMessage(text: string): Promise<void> {
    const trimmed = text.trim();
    if (!trimmed) return;
    if (!store.activeSessionId) await startSession();
    const id = store.activeSessionId;
    if (!id) return;
    const idx = store.sessions.findIndex((s) => s.info.id === id);
    if (idx >= 0) {
      setStore(
        "sessions",
        idx,
        produce((s) => {
          finalizeLiveAssistant(s, "completed"); // mid-turn steering: close the live message
          s.messages.push({ id: `opt-${crypto.randomUUID()}`, role: "user", content: trimmed, blocks: [] });
          s.runState = "generating";
        }),
      );
    }
    try {
      await ipc.sendSessionInput(id, trimmed);
    } catch (e) {
      pushError(String(e));
      if (idx >= 0) setStore("sessions", idx, "runState", "error");
    }
  }

  /** Kept name from the scaffold; same behavior as sendMessage. */
  function sendSessionInput(text: string): Promise<void> {
    return sendMessage(text);
  }

  async function approveRequest(sessionId: string, requestId: string, approve: boolean): Promise<void> {
    try {
      await ipc.approveSession(sessionId, requestId, approve);
    } catch (e) {
      pushError(String(e));
    }
    const idx = store.sessions.findIndex((s) => s.info.id === sessionId);
    if (idx >= 0 && store.sessions[idx].pendingApproval?.requestId === requestId) {
      setStore("sessions", idx, "pendingApproval", null);
    }
  }

  async function stopSession(id: string): Promise<void> {
    try {
      await ipc.stopSession(id);
    } catch (e) {
      pushError(String(e));
    }
    setStore("sessions", (s) => s.filter((x) => x.info.id !== id));
    if (store.activeSessionId === id) setStore("activeSessionId", null);
  }

  /** "Ask Claude" flow: seed the composer and reveal the session pane. */
  function prefillComposer(text: string): void {
    setStore({ composerPrefill: text, sessionPaneOpen: true });
  }

  function clearComposerPrefill(): void {
    setStore("composerPrefill", null);
  }

  return {
    startSession,
    setSessionMode,
    sendMessage,
    sendSessionInput,
    approveRequest,
    stopSession,
    prefillComposer,
    clearComposerPrefill,
    handleAgentEvent: createAgentEventHandler(store, setStore, pushError),
  };
}
