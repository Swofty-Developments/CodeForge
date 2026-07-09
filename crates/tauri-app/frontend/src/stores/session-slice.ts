/**
 * Session slice — start/send/approve/stop actions plus the `agent-event`
 * reducer (CodeForge streaming model, docs/ARCHITECTURE.md §Sessions).
 * Demux: payload.sessionId matches SessionInfo.id; payload.threadId carries
 * the Claude SDK session id and matches claudeSessionId as a fallback.
 */

import { produce, type SetStoreFunction } from "solid-js/store";
import * as ipc from "../ipc";
import type {
  AgentEventPayload,
  ContentBlock,
  SessionInfo,
  SessionMessage,
  SessionUi,
} from "../types";
import type { AppStore } from "./app-store";

function newSessionUi(info: SessionInfo): SessionUi {
  return {
    info,
    runState: "starting",
    blocks: [],
    messages: [],
    pendingApproval: null,
    slashCommands: [],
    claudeSessionId: null,
    usage: { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheWriteTokens: 0, costUsd: 0 },
  };
}

/** The live streaming message = last assistant message without a `done-` id. */
function findLiveAssistant(s: SessionUi): SessionMessage | null {
  for (let i = s.messages.length - 1; i >= 0; i--) {
    const m = s.messages[i];
    if (m.role === "assistant" && !m.id.startsWith("done-")) return m;
  }
  return null;
}

function ensureLiveAssistant(s: SessionUi): SessionMessage {
  const live = findLiveAssistant(s);
  if (live) return live;
  const msg: SessionMessage = { id: crypto.randomUUID(), role: "assistant", content: "", blocks: [] };
  s.messages.push(msg);
  return msg;
}

/** Backwards scan; an empty toolId matches the most recent tool block. */
function findToolBlock(blocks: ContentBlock[], toolId: string | undefined): ContentBlock | null {
  for (let i = blocks.length - 1; i >= 0; i--) {
    const b = blocks[i];
    if (b.type === "tool_use" && (!toolId || b.toolId === toolId)) return b;
  }
  return null;
}

/** Push into both the message and the session-wide flat mirror (shared object). */
function pushBlock(s: SessionUi, msg: SessionMessage, block: ContentBlock): void {
  msg.blocks.push(block);
  s.blocks.push(block);
}

function finalizeLiveAssistant(s: SessionUi, toolOutcome: "completed" | "error"): void {
  const msg = findLiveAssistant(s);
  if (!msg) return;
  for (const b of msg.blocks) {
    if (b.type === "tool_use" && (b.toolStatus === "generating" || b.toolStatus === "running")) {
      b.toolStatus = toolOutcome;
      if (toolOutcome === "error") b.toolError = true;
    }
  }
  msg.content = msg.blocks.filter((b) => b.type === "text").map((b) => b.content).join("\n");
  msg.id = `done-${crypto.randomUUID()}`;
}

function pushSystemMessage(s: SessionUi, content: string): void {
  s.messages.push({ id: `done-${crypto.randomUUID()}`, role: "system", content, blocks: [] });
}

export function createSessionSlice(
  store: AppStore,
  setStore: SetStoreFunction<AppStore>,
  pushError: (message: string) => void,
) {
  async function startSession(model?: string): Promise<void> {
    if (!store.repo) return;
    try {
      const info = await ipc.startSession({ repoPath: store.repo.path, model });
      setStore("sessions", store.sessions.length, newSessionUi(info));
      setStore("activeSessionId", info.id);
    } catch (e) {
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

  function handleAgentEvent(payload: AgentEventPayload): void {
    const idx = store.sessions.findIndex(
      (s) =>
        s.info.id === payload.sessionId ||
        (!!payload.threadId && s.claudeSessionId === payload.threadId),
    );
    if (idx < 0) return;
    const mutate = (fn: (s: SessionUi) => void) => setStore("sessions", idx, produce(fn));

    switch (payload.eventType) {
      case "content_delta": {
        const text = payload.text ?? "";
        if (!text) return;
        mutate((s) => {
          const msg = ensureLiveAssistant(s);
          const last = msg.blocks[msg.blocks.length - 1];
          if (last && last.type === "text") last.content += text;
          else pushBlock(s, msg, { type: "text", content: text });
          msg.content += text;
        });
        break;
      }
      case "thinking_delta": {
        const text = payload.text ?? "";
        if (!text) return;
        mutate((s) => {
          const msg = ensureLiveAssistant(s);
          const last = msg.blocks[msg.blocks.length - 1];
          if (last && last.type === "thinking") last.content += text;
          else pushBlock(s, msg, { type: "thinking", content: text });
        });
        break;
      }
      case "tool_use_start":
        mutate((s) =>
          pushBlock(s, ensureLiveAssistant(s), {
            type: "tool_use",
            content: "",
            toolId: payload.toolId ?? "",
            toolName: payload.toolName ?? "tool",
            toolInput: "",
            toolStatus: "generating",
          }),
        );
        break;
      case "tool_input_delta":
        mutate((s) => {
          const b = findToolBlock(s.blocks, payload.toolId);
          if (b) b.toolInput = (b.toolInput ?? "") + (payload.inputJson ?? "");
        });
        break;
      case "tool_use_end":
        mutate((s) => {
          const b = findToolBlock(s.blocks, payload.toolId);
          if (b) b.toolStatus = "running";
        });
        break;
      case "tool_result":
        mutate((s) => {
          const b = findToolBlock(s.blocks, payload.toolId);
          if (!b) return;
          b.toolOutput = payload.toolOutput ?? "";
          b.toolStatus = payload.isError ? "error" : "completed";
          b.toolError = !!payload.isError;
          // Claude tool_results arrive with an empty toolName; the block keeps its own.
          if (payload.toolName) b.toolName = payload.toolName;
        });
        break;
      case "turn_started":
        mutate((s) => {
          s.runState = "generating";
        });
        break;
      case "turn_completed":
        mutate((s) => {
          if (payload.turnId && !s.claudeSessionId) s.claudeSessionId = payload.turnId;
          finalizeLiveAssistant(s, "completed");
          s.runState = "ready";
        });
        break;
      case "turn_aborted":
        mutate((s) => {
          finalizeLiveAssistant(s, "error");
          pushSystemMessage(s, `Aborted: ${payload.reason ?? "unknown"}`);
          s.runState = "ready";
        });
        break;
      case "usage_report":
        mutate((s) => {
          s.usage.inputTokens += payload.inputTokens ?? 0;
          s.usage.outputTokens += payload.outputTokens ?? 0;
          s.usage.cacheReadTokens += payload.cacheReadTokens ?? 0;
          s.usage.cacheWriteTokens += payload.cacheWriteTokens ?? 0;
          s.usage.costUsd += payload.costUsd ?? 0;
          for (let i = s.messages.length - 1; i >= 0; i--) {
            const m = s.messages[i];
            if (m.role === "assistant") {
              m.meta = {
                model: payload.model,
                inputTokens: payload.inputTokens,
                outputTokens: payload.outputTokens,
                costUsd: payload.costUsd,
              };
              break;
            }
          }
        });
        break;
      case "session_ready":
        mutate((s) => {
          // payload.message carries the Claude SDK session id (forge-session payload.rs).
          if (payload.message) s.claudeSessionId = payload.message;
          if (payload.model) s.info.model = payload.model;
          // init arrives mid-turn with `claude -p`; never downgrade "generating".
          if (s.runState !== "generating") s.runState = "ready";
        });
        break;
      case "slash_commands":
        mutate((s) => {
          s.slashCommands = payload.commands ?? [];
        });
        break;
      case "session_error":
        mutate((s) => {
          s.runState = "error";
          pushSystemMessage(s, `Error: ${payload.message ?? "unknown"}`);
        });
        pushError(payload.message ?? "Session error");
        break;
      case "approval_required":
        mutate((s) => {
          s.pendingApproval = {
            requestId: payload.requestId ?? "",
            description: payload.description ?? "",
          };
        });
        break;
      default:
        break;
    }
  }

  return {
    startSession,
    sendMessage,
    sendSessionInput,
    approveRequest,
    stopSession,
    prefillComposer,
    clearComposerPrefill,
    handleAgentEvent,
  };
}
