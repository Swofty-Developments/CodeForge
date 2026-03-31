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

  // canUseTool callback — sends approval requests to Rust, waits for response
  options.canUseTool = async (toolName, input) => {
    const requestId = String(++approvalCounter);

    // AskUserQuestion — forward to frontend
    if (toolName === "AskUserQuestion") {
      emit({
        type: "ask_user_question",
        requestId,
        questions: input.questions || [],
      });
    } else {
      // Regular tool approval
      emit({
        type: "approval_request",
        requestId,
        toolName,
        input,
      });
    }

    // Wait for the response from Rust
    return new Promise((resolve) => {
      pendingApprovals.set(requestId, {
        resolve: (resp) => {
          if (resp.decision === "allow") {
            resolve({ behavior: "allow", updatedInput: input });
          } else {
            resolve({ behavior: "deny", message: resp.message || "User denied this action" });
          }
        },
      });
    });
  };

  // Create an AbortController for this query.
  const abort = new AbortController();
  currentAbort = abort;
  options.signal = abort.signal;

  let capturedSessionId = sessionId || null;
  let turnEmitted = false;

  // Emit turn_started so the frontend shows "generating" state
  emit({ type: "turn_started" });

  try {
    for await (const message of query({ prompt, options })) {
      // Abort was requested while iterating.
      if (abort.signal.aborted) break;

      const msgType = message.type;

      // ── system messages ──
      if (msgType === "system") {
        if (message.subtype === "init" && message.session_id) {
          capturedSessionId = message.session_id;
          emit({ type: "session_ready", sessionId: message.session_id });
        }
        continue;
      }

      // ── result message (final) ──
      if (msgType === "result" || "result" in message) {
        // Extract usage from various possible locations
        const usage = message.usage || message.modelUsage;
        const costUsd = message.total_cost_usd ?? message.cost_usd ?? 0;
        const modelName = message.model ?? model ?? "unknown";

        if (usage) {
          // SDK may nest usage per-model or flat
          let inputTokens = 0, outputTokens = 0, cacheRead = 0, cacheWrite = 0;

          if (typeof usage === "object" && !Array.isArray(usage)) {
            // Check if it's a flat usage object or per-model
            if (usage.input_tokens != null || usage.inputTokens != null) {
              inputTokens = usage.input_tokens ?? usage.inputTokens ?? 0;
              outputTokens = usage.output_tokens ?? usage.outputTokens ?? 0;
              cacheRead = usage.cache_read_input_tokens ?? usage.cacheReadInputTokens ?? 0;
              cacheWrite = usage.cache_creation_input_tokens ?? usage.cacheCreationInputTokens ?? 0;
            } else {
              // Per-model usage: { "claude-...": { inputTokens, ... } }
              for (const [, modelUsage] of Object.entries(usage)) {
                if (typeof modelUsage === "object") {
                  inputTokens += modelUsage.inputTokens ?? 0;
                  outputTokens += modelUsage.outputTokens ?? 0;
                  cacheRead += modelUsage.cacheReadInputTokens ?? 0;
                  cacheWrite += modelUsage.cacheCreationInputTokens ?? 0;
                }
              }
            }
          }

          emit({
            type: "usage",
            inputTokens,
            outputTokens,
            cacheRead,
            cacheWrite,
            costUsd,
            model: modelName,
          });
        }

        emit({ type: "turn_completed", sessionId: capturedSessionId || "" });
        turnEmitted = true;
        continue;
      }

      // ── streaming content messages ──
      // The SDK yields various message shapes. We normalise them to our protocol.

      // stream_event wrapping Anthropic API SSE events
      if (msgType === "stream_event" && message.event) {
        handleStreamEvent(message.event);
        continue;
      }

      // assistant message — extract content blocks
      if (msgType === "assistant" && message.message?.content) {
        for (const block of message.message.content) {
          if (block.type === "text" && block.text) {
            emit({ type: "text_delta", text: block.text });
          } else if (block.type === "tool_use") {
            emit({
              type: "tool_use_start",
              toolId: block.id ?? "",
              toolName: block.name ?? "tool",
            });
            if (block.input) {
              emit({
                type: "tool_use_input",
                toolId: block.id ?? "",
                inputJson: JSON.stringify(block.input),
              });
            }
          } else if (block.type === "thinking" && block.thinking) {
            emit({ type: "thinking_delta", text: block.thinking });
          }
        }
        continue;
      }

      // user message — contains tool results
      if (msgType === "user" && message.message?.content) {
        for (const block of message.message.content) {
          if (block.type === "tool_result") {
            const content = typeof block.content === "string"
              ? block.content
              : JSON.stringify(block.content);
            emit({
              type: "tool_result",
              toolId: block.tool_use_id ?? "",
              toolName: "",
              content,
              isError: !!block.is_error,
            });
          }
        }
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
