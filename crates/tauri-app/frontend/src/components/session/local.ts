/**
 * Session-pane side-channel state that the frozen AppStore shape cannot hold:
 * user/system entries interleaved into the block stream, per-session usage
 * totals, and helpers that mirror missing store actions (see integration notes).
 */

import { createRoot } from "solid-js";
import { createStore, produce } from "solid-js/store";
import * as ipc from "../../ipc";
import { appStore } from "../../stores/app-store";
import type { AgentEventPayload } from "../../types";

export interface StreamEntry {
  kind: "user" | "system";
  text: string;
  severity: "info" | "warn" | "error";
  /** Index into SessionUi.blocks this entry renders before (blocks.length = end). */
  at: number;
}

export interface SessionUsage {
  inputTokens: number;
  outputTokens: number;
  model: string;
}

interface SessionLocal {
  entries: StreamEntry[];
  usage: SessionUsage | null;
}

const EMPTY: SessionLocal = { entries: [], usage: null };

const [locals, setLocals] = createRoot(() => createStore<Record<string, SessionLocal>>({}));

export function sessionLocal(sessionId: string): SessionLocal {
  return locals[sessionId] ?? EMPTY;
}

function ensure(sessionId: string): void {
  if (!locals[sessionId]) setLocals(sessionId, { entries: [], usage: null });
}

function blocksLen(sessionId: string): number {
  return appStore.store.sessions.find((s) => s.info.id === sessionId)?.blocks.length ?? 0;
}

export function recordUserEntry(sessionId: string, text: string): void {
  ensure(sessionId);
  const entry: StreamEntry = { kind: "user", text, severity: "info", at: blocksLen(sessionId) };
  setLocals(sessionId, "entries", (e) => [...e, entry]);
}

function recordSystemEntry(sessionId: string, text: string, severity: "warn" | "error"): void {
  ensure(sessionId);
  const entry: StreamEntry = { kind: "system", text, severity, at: blocksLen(sessionId) };
  setLocals(sessionId, "entries", (e) => [...e, entry]);
}

function handleEvent(p: AgentEventPayload): void {
  switch (p.eventType) {
    case "usage_report": {
      ensure(p.sessionId);
      const prev = locals[p.sessionId].usage;
      setLocals(p.sessionId, "usage", {
        inputTokens: (prev?.inputTokens ?? 0) + (p.inputTokens ?? 0),
        outputTokens: (prev?.outputTokens ?? 0) + (p.outputTokens ?? 0),
        model: p.model ?? prev?.model ?? "",
      });
      break;
    }
    case "turn_aborted":
      recordSystemEntry(p.sessionId, `Aborted: ${p.reason ?? "interrupted"}`, "warn");
      break;
    case "session_error":
      // slash_commands smuggling channel — handled by the store reducer.
      if (p.message?.startsWith("slash_commands:")) break;
      recordSystemEntry(p.sessionId, p.message ?? "Session error", "error");
      break;
    default:
      break;
  }
}

// Second module-level listener beside the store's (main.tsx) so a collapsed
// pane still accumulates usage/system entries. Rejects outside Tauri — fine.
void ipc.listenAgentEvent(handleEvent).catch(() => {});

// ── Store-gap helpers (mirror actions the frozen store doesn't expose yet) ──

/** Start a session at a chosen model; the store builds the full SessionUi. */
export async function startSessionWithModel(model?: string): Promise<void> {
  await appStore.startSession(model);
}

export function selectSession(sessionId: string): void {
  appStore.setStore("activeSessionId", sessionId);
}

/** Stops the session; keeps another tab focused; frees local side-state. */
export async function closeSession(sessionId: string): Promise<void> {
  await appStore.stopSession(sessionId);
  setLocals(produce((l) => delete l[sessionId]));
  const { store, setStore } = appStore;
  if (store.activeSessionId === null && store.sessions.length > 0) {
    setStore("activeSessionId", store.sessions[store.sessions.length - 1].info.id);
  }
}

/** The store's approveRequest never clears pendingApproval — do it here. */
export function clearApproval(sessionId: string): void {
  const idx = appStore.store.sessions.findIndex((s) => s.info.id === sessionId);
  if (idx >= 0) appStore.setStore("sessions", idx, "pendingApproval", null);
}
