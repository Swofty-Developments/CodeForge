/**
 * The `agent-event` reducer (CodeForge streaming model) + the message helpers it
 * shares with the session actions. Demux: payload.sessionId is the routing key
 * (CONTRACT-1) — the backend stamps it on every event. `messages` is the single
 * source of truth; there is no parallel block mirror.
 */

import { produce, type SetStoreFunction } from "solid-js/store";
import type { AgentEventPayload, ContentBlock, SessionMessage, SessionUi } from "../types";
import type { AppStore } from "./app-store";

/** The live streaming message = last assistant message without a `done-` id. */
export function findLiveAssistant(s: SessionUi): SessionMessage | null {
  for (let i = s.messages.length - 1; i >= 0; i--) {
    const m = s.messages[i];
    if (m.role === "assistant" && !m.id.startsWith("done-")) return m;
  }
  return null;
}

export function ensureLiveAssistant(s: SessionUi): SessionMessage {
  const live = findLiveAssistant(s);
  if (live) return live;
  const msg: SessionMessage = { id: crypto.randomUUID(), role: "assistant", content: "", blocks: [] };
  s.messages.push(msg);
  return msg;
}

/** Backwards scan across messages; an empty toolId matches the most recent
 *  tool block. Tool blocks live on the live assistant message. */
export function findToolBlock(s: SessionUi, toolId: string | undefined): ContentBlock | null {
  for (let mi = s.messages.length - 1; mi >= 0; mi--) {
    const blocks = s.messages[mi].blocks;
    for (let bi = blocks.length - 1; bi >= 0; bi--) {
      const b = blocks[bi];
      if (b.type === "tool_use" && (!toolId || b.toolId === toolId)) return b;
    }
  }
  return null;
}

export function finalizeLiveAssistant(s: SessionUi, toolOutcome: "completed" | "error"): void {
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

export function pushSystemMessage(s: SessionUi, content: string, level: "info" | "warn" | "error" = "info"): void {
  s.messages.push({ id: `done-${crypto.randomUUID()}`, role: "system", content, blocks: [], level });
}

export function createAgentEventHandler(
  store: AppStore,
  setStore: SetStoreFunction<AppStore>,
  pushError: (message: string) => void,
) {
  return function handleAgentEvent(payload: AgentEventPayload): void {
    // CONTRACT-1: route strictly by sessionId (stamped on every event).
    const idx = store.sessions.findIndex((s) => s.info.id === payload.sessionId);
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
          else msg.blocks.push({ type: "text", content: text });
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
          else msg.blocks.push({ type: "thinking", content: text });
        });
        break;
      }
      case "tool_use_start":
        mutate((s) =>
          ensureLiveAssistant(s).blocks.push({
            type: "tool_use",
            content: "",
            toolId: payload.toolId ?? "",
            // Keep nullable — the UI renders an explicit "unknown tool" affordance
            // rather than fabricating a name.
            toolName: payload.toolName,
            toolInput: "",
            toolStatus: "generating",
          }),
        );
        break;
      case "tool_input_delta":
        mutate((s) => {
          const b = findToolBlock(s, payload.toolId);
          if (b) b.toolInput = (b.toolInput ?? "") + (payload.inputJson ?? "");
        });
        break;
      case "tool_use_end":
        mutate((s) => {
          const b = findToolBlock(s, payload.toolId);
          if (b) b.toolStatus = "running";
        });
        break;
      case "tool_result":
        mutate((s) => {
          const b = findToolBlock(s, payload.toolId);
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
          pushSystemMessage(s, `Aborted: ${payload.reason ?? "unknown"}`, "warn");
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
          pushSystemMessage(s, `Error: ${payload.message ?? "unknown"}`, "error");
        });
        pushError(payload.message ?? "Session error");
        break;
      // A resume attempt failed — a distinct, surfaced state (not a silent
      // fall-through to a fresh session). Agent A emits the detail in `message`.
      case "session_resume_failed":
        mutate((s) => {
          s.runState = "error";
          pushSystemMessage(s, `Resume failed: ${payload.message ?? "unknown"}`, "error");
        });
        pushError(`Session resume failed: ${payload.message ?? "unknown"}`);
        break;
      // The session runs, but server-side persistence is degraded — shown
      // honestly rather than hidden behind the happy path.
      case "session_persistence_degraded":
        mutate((s) => {
          pushSystemMessage(s, `Persistence degraded: ${payload.message ?? "unknown"}`, "warn");
        });
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
  };
}
