/* Scroll-anchored message stream for one session. Renders straight from
 * `session.messages` (the single source of truth): user bubbles, system pills,
 * and assistant messages whose ContentBlocks render as text / thinking / tool
 * cards. Plus the typing indicator, approval card, and usage footer. */

import { For, Match, Show, Switch, createEffect, createMemo, on, onCleanup } from "solid-js";
import { ApprovalCard, ThinkingBlock, ToolCard, TypingIndicator } from "./blocks";
import { Markdown } from "./markdown";
import type { ContentBlock, SessionMessage, SessionUi } from "../../types";

function fmtTokens(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n);
}

export function MessageStream(props: { session: SessionUi }) {
  const messages = () => props.session.messages;
  const generating = () => props.session.runState === "generating";
  const lastMsgIndex = createMemo(() => messages().length - 1);

  // Live text streams inline into the trailing assistant block; anything else
  // (user just sent, a tool is running) shows the typing indicator instead.
  const showTyping = () => {
    if (!generating()) return false;
    const last = messages()[lastMsgIndex()];
    if (!last || last.role !== "assistant") return true;
    const b = last.blocks;
    return b.length === 0 || b[b.length - 1].type !== "text";
  };

  const isTrailingBlock = (mi: number, bi: number, blocks: ContentBlock[]) =>
    mi === lastMsgIndex() && bi === blocks.length - 1;

  // ── Scroll anchoring: 150px away-threshold, 80ms debounce ─────────────────
  let listRef: HTMLDivElement | undefined;
  let scrolledAway = false;
  let scrollTimer: number | null = null;

  function onScroll(): void {
    if (!listRef) return;
    scrolledAway = listRef.scrollHeight - listRef.scrollTop - listRef.clientHeight > 150;
  }

  function scheduleScroll(): void {
    if (scrollTimer !== null) return;
    scrollTimer = window.setTimeout(() => {
      scrollTimer = null;
      if (!scrolledAway && listRef) listRef.scrollTop = listRef.scrollHeight;
    }, 80);
  }

  onCleanup(() => {
    if (scrollTimer !== null) window.clearTimeout(scrollTimer);
  });

  // Track everything that grows the stream, then schedule a follow.
  createEffect(() => {
    const s = props.session;
    s.runState;
    s.pendingApproval;
    for (const m of s.messages) {
      for (const b of m.blocks) {
        b.content;
        b.toolOutput;
        b.toolStatus;
      }
    }
    s.usage.outputTokens;
    scheduleScroll();
  });

  // Switching sessions or adding a message re-anchors to the bottom.
  createEffect(
    on(() => props.session.info.id, () => {
      scrolledAway = false;
      if (listRef) listRef.scrollTop = listRef.scrollHeight;
    })
  );
  createEffect(
    on(() => props.session.messages.length, () => {
      scrolledAway = false;
    })
  );

  const showUsage = () => props.session.usage.inputTokens > 0 || props.session.usage.outputTokens > 0;

  return (
    <div class="sp-stream" ref={listRef} onScroll={onScroll}>
      <Show when={messages().length === 0 && !generating()}>
        <div class="sp-stream-hint">
          Session ready — ask for work below. Hooks feed the timeline as Claude edits.
        </div>
      </Show>

      <For each={messages()}>
        {(msg, mi) => (
          <Switch>
            <Match when={msg.role === "user"}>
              <div class="msg-user-bubble">{msg.content}</div>
            </Match>
            <Match when={msg.role === "system"}>
              <div class="msg-system-row">
                <span
                  class="msg-system-pill"
                  classList={{
                    "msg-system-pill--warn": msg.level === "warn",
                    "msg-system-pill--error": msg.level === "error",
                  }}
                >
                  {msg.content}
                </span>
              </div>
            </Match>
            <Match when={msg.role === "assistant"}>
              <AssistantMessage msg={msg} messageIndex={mi()} isTrailingBlock={isTrailingBlock} generating={generating} />
            </Match>
          </Switch>
        )}
      </For>

      <Show when={showTyping()}>
        <TypingIndicator />
      </Show>

      <Show when={props.session.pendingApproval}>
        {(approval) => <ApprovalCard sessionId={props.session.info.id} approval={approval()} />}
      </Show>

      <Show when={showUsage()}>
        <div class="sp-usage">
          {fmtTokens(props.session.usage.inputTokens)} in · {fmtTokens(props.session.usage.outputTokens)} out
          <Show when={props.session.info.model}> · {props.session.info.model}</Show>
        </div>
      </Show>
    </div>
  );
}

function AssistantMessage(props: {
  msg: SessionMessage;
  messageIndex: number;
  isTrailingBlock: (mi: number, bi: number, blocks: ContentBlock[]) => boolean;
  generating: () => boolean;
}) {
  const streaming = (bi: number) =>
    props.generating() && props.isTrailingBlock(props.messageIndex, bi, props.msg.blocks);

  return (
    <For each={props.msg.blocks}>
      {(block, bi) => (
        <Switch>
          <Match when={block.type === "text"}>
            <div class="msg-assistant">
              <Markdown content={block.content} streaming={streaming(bi())} />
            </div>
          </Match>
          <Match when={block.type === "thinking"}>
            <ThinkingBlock block={block} streaming={streaming(bi())} />
          </Match>
          <Match when={block.type === "tool_use"}>
            <ToolCard block={block} />
          </Match>
        </Switch>
      )}
    </For>
  );
}
