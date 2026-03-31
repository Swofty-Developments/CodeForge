#!/usr/bin/env node

/**
 * CodeForge Agent Sidecar
 *
 * Node.js process that wraps the @anthropic-ai/claude-agent-sdk `query()` function.
 * Communicates with the Rust backend via NDJSON over stdin/stdout.
 *
 * Stdin commands:
 *   { type: "query", prompt, cwd, model?, permissionMode?, sessionId?, allowedTools? }
 *   { type: "approval_response", requestId, decision, message? }
 *   { type: "abort" }
 *
 * Stdout events:
 *   { type: "ready" }
 *   { type: "text_delta", text }
 *   { type: "tool_use_start", toolId, toolName }
 *   { type: "tool_use_input", toolId, inputJson }
 *   { type: "tool_result", toolId, toolName, content, isError }
 *   { type: "thinking_delta", text }
 *   { type: "approval_request", requestId, toolName, input }
 *   { type: "ask_user_question", requestId, questions }
 *   { type: "session_ready", sessionId }
 *   { type: "turn_completed", sessionId }
 *   { type: "usage", inputTokens, outputTokens, cacheRead, cacheWrite, costUsd, model }
 *   { type: "error", message }
 */

import { query } from "@anthropic-ai/claude-agent-sdk";
import { createInterface } from "readline";

// ── Helpers ──────────────────────────────────────────────────────────────────

function emit(obj) {
  try {
    process.stdout.write(JSON.stringify(obj) + "\n");
  } catch (_) {
    // stdout may be closed if the parent died
  }
}

// ── State ────────────────────────────────────────────────────────────────────

// Pending approval callbacks keyed by requestId.
// Each entry: { resolve: (decision) => void }
const pendingApprovals = new Map();

// AbortController for the current query, if any.
let currentAbort = null;

// Counter for generating unique approval request IDs.
let approvalCounter = 0;

// ── Stdin reader ─────────────────────────────────────────────────────────────

const rl = createInterface({ input: process.stdin, terminal: false });

rl.on("line", (line) => {
  if (!line.trim()) return;

  let cmd;
  try {
    cmd = JSON.parse(line);
  } catch {
    emit({ type: "error", message: `Invalid JSON on stdin: ${line}` });
    return;
  }

  switch (cmd.type) {
    case "query":
      handleQuery(cmd).catch((err) => {
        emit({ type: "error", message: String(err?.message ?? err) });
      });
      break;

    case "approval_response":
      handleApprovalResponse(cmd);
      break;

    case "abort":
      handleAbort();
      break;

    default:
      emit({ type: "error", message: `Unknown command type: ${cmd.type}` });
  }
});

rl.on("close", () => {
  // Parent closed stdin — exit cleanly.
  process.exit(0);
});

// ── Command handlers ─────────────────────────────────────────────────────────

async function handleQuery(cmd) {
  const {
    prompt,
    cwd,
    model,
    permissionMode,
    sessionId,
    allowedTools,
  } = cmd;

  // Change to the requested working directory.
  if (cwd) {
    try {
      process.chdir(cwd);
    } catch (err) {
      emit({ type: "error", message: `Failed to chdir to ${cwd}: ${err.message}` });
      return;
    }
  }

  // Build query options.
  const options = {};

  if (cwd) options.cwd = cwd;

  if (allowedTools && Array.isArray(allowedTools) && allowedTools.length > 0) {
    options.allowedTools = allowedTools;
  }

  if (permissionMode) {
    options.permissionMode = permissionMode;
    if (permissionMode === "bypassPermissions") {
      options.allowDangerouslySkipPermissions = true;
    }
  }

  if (model) options.model = model;

  // Resume support: pass the previous session ID.
  if (sessionId) {
    options.resume = sessionId;
  }

  // Create an AbortController for this query.
  const abort = new AbortController();
  currentAbort = abort;
  options.signal = abort.signal;

  let capturedSessionId = sessionId || null;
  let turnEmitted = false;

  try {
    for await (const message of query({ prompt, options })) {
      // Abort was requested while iterating.
      if (abort.signal.aborted) break;

      // ── system messages ──
      if (message.type === "system") {
        if (message.subtype === "init" && message.session_id) {
          capturedSessionId = message.session_id;
          emit({ type: "session_ready", sessionId: message.session_id });
        }
        continue;
      }

      // ── result message (final) ──
      if ("result" in message) {
        // Extract usage if available.
        if (message.usage) {
          emit({
            type: "usage",
            inputTokens: message.usage.input_tokens ?? 0,
            outputTokens: message.usage.output_tokens ?? 0,
            cacheRead: message.usage.cache_read_input_tokens ?? message.usage.cache_read_tokens ?? 0,
            cacheWrite: message.usage.cache_creation_input_tokens ?? message.usage.cache_write_tokens ?? 0,
            costUsd: message.cost_usd ?? message.total_cost_usd ?? 0,
            model: message.model ?? model ?? "unknown",
          });
        }

        emit({ type: "turn_completed", sessionId: capturedSessionId || "" });
        turnEmitted = true;
        continue;
      }

      // ── streaming content messages ──
      // The SDK yields various message shapes. We normalise them to our protocol.

      const msgType = message.type;

      // stream_event wrapping Anthropic API SSE events
      if (msgType === "stream_event" && message.event) {
        handleStreamEvent(message.event);
        continue;
      }

      // assistant message snapshots (full content)
      if (msgType === "assistant" && message.message?.content) {
        // We prefer stream_event deltas; skip snapshot processing.
        continue;
      }

      // Content delta shorthand (some SDK versions)
      if (msgType === "content_delta" || msgType === "text") {
        const text = message.text ?? message.delta?.text ?? "";
        if (text) emit({ type: "text_delta", text });
        continue;
      }
    }
  } catch (err) {
    if (abort.signal.aborted) {
      // Intentional abort — not an error.
    } else {
      emit({ type: "error", message: String(err?.message ?? err) });
    }
  } finally {
    currentAbort = null;

    // Make sure we always emit turn_completed so the Rust side knows we're done.
    if (!turnEmitted) {
      emit({ type: "turn_completed", sessionId: capturedSessionId || "" });
    }
  }
}

/**
 * Handle raw Anthropic API SSE events forwarded by the SDK.
 */
function handleStreamEvent(event) {
  const eventType = event.type;

  switch (eventType) {
    case "message_start": {
      // Could extract model here if needed.
      break;
    }

    case "content_block_start": {
      const block = event.content_block;
      if (!block) break;

      if (block.type === "tool_use") {
        emit({
          type: "tool_use_start",
          toolId: block.id ?? "",
          toolName: block.name ?? "tool",
        });
      }
      break;
    }

    case "content_block_delta": {
      const delta = event.delta;
      if (!delta) break;

      switch (delta.type) {
        case "text_delta":
          if (delta.text) emit({ type: "text_delta", text: delta.text });
          break;
        case "input_json_delta":
          if (delta.partial_json != null) {
            // We need to know which tool this belongs to. The SDK usually
            // provides event.index; we map it via prior content_block_start.
            emit({
              type: "tool_use_input",
              toolId: "", // will be correlated by order on the Rust side
              inputJson: delta.partial_json,
            });
          }
          break;
        case "thinking_delta":
          if (delta.thinking) emit({ type: "thinking_delta", text: delta.thinking });
          break;
      }
      break;
    }

    case "content_block_end": {
      // We could emit tool_use_end here, but the Rust side tracks this
      // via tool_result events already.
      break;
    }

    case "message_delta": {
      // Contains stop_reason and usage deltas.
      break;
    }
  }
}

function handleApprovalResponse(cmd) {
  const { requestId, decision, message } = cmd;
  const pending = pendingApprovals.get(requestId);
  if (pending) {
    pendingApprovals.delete(requestId);
    pending.resolve({ decision, message });
  }
}

function handleAbort() {
  if (currentAbort) {
    currentAbort.abort();
  }
}

// ── Ready ────────────────────────────────────────────────────────────────────

emit({ type: "ready" });
