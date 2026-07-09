/* Scroll-anchored message stream for one session: user bubbles + system pills
 * (local side-channel) interleaved with the store's ContentBlocks, typing
 * indicator, approval card, and the usage footer line. */

import { Index, Match, Show, Switch, createEffect, createMemo, on, onCleanup } from "solid-js";
import { ApprovalCard, ThinkingBlock, ToolCard, TypingIndicator } from "./blocks";
import { Markdown } from "./markdown";
import { sessionLocal, type StreamEntry } from "./local";
import type { ContentBlock, SessionUi } from "../../types";

type StreamItem =
  | { kind: "entry"; entry: StreamEntry }
  | { kind: "block"; block: ContentBlock; index: number };

function fmtTokens(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n);
}

export function MessageStream(props: { session: SessionUi }) {
  const local = () => sessionLocal(props.session.info.id);

  const items = createMemo<StreamItem[]>(() => {
    const blocks = props.session.blocks;
    const entries = local().entries;
    const out: StreamItem[] = [];
    let ei = 0;
    for (let i = 0; i < blocks.length; i++) {
      while (ei < entries.length && entries[ei].at <= i) out.push({ kind: "entry", entry: entries[ei++] });
      out.push({ kind: "block", block: blocks[i], index: i });
    }
    while (ei < entries.length) out.push({ kind: "entry", entry: entries[ei++] });
    return out;
  });

  const generating = () => props.session.runState === "generating";
  const isLastBlock = (index: number) => index === props.session.blocks.length - 1;
  const showTyping = () => {
    if (!generating()) return false;
    const blocks = props.session.blocks;
    return blocks.length === 0 || blocks[blocks.length - 1].type !== "text";
  };

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
    for (const b of s.blocks) {
      b.content;
      b.toolOutput;
      b.toolStatus;
    }
    local().entries.length;
    local().usage;
    scheduleScroll();
  });

  // Switching sessions or sending a new message re-anchors to the bottom.
  createEffect(
    on(() => props.session.info.id, () => {
      scrolledAway = false;
      if (listRef) listRef.scrollTop = listRef.scrollHeight;
    })
  );
  createEffect(
    on(() => local().entries.length, () => {
      scrolledAway = false;
    })
  );

  return (
    <div class="sp-stream" ref={listRef} onScroll={onScroll}>
      <Show when={items().length === 0 && !generating()}>
        <div class="sp-stream-hint">
          Session ready — ask for work below. Hooks feed the timeline as Claude edits.
        </div>
      </Show>

      <Index each={items()}>
        {(item) => {
          // Defensive accessors — an item at a given index can change shape
          // when an entry is inserted, so every branch re-checks its guard.
          const userEntry = () => {
            const it = item();
            return it.kind === "entry" && it.entry.kind === "user" ? it.entry : null;
          };
          const sysEntry = () => {
            const it = item();
            return it.kind === "entry" && it.entry.kind === "system" ? it.entry : null;
          };
          const blockOf = (type: ContentBlock["type"]) => {
            const it = item();
            return it.kind === "block" && it.block.type === type ? it : null;
          };
          return (
            <Switch>
              <Match when={userEntry()}>
                {(e) => <div class="msg-user-bubble">{e().text}</div>}
              </Match>
              <Match when={sysEntry()}>
                {(e) => (
                  <div class="msg-system-row">
                    <span
                      class="msg-system-pill"
                      classList={{
                        "msg-system-pill--warn": e().severity === "warn",
                        "msg-system-pill--error": e().severity === "error",
                      }}
                    >
                      {e().text}
                    </span>
                  </div>
                )}
              </Match>
              <Match when={blockOf("text")}>
                {(b) => (
                  <div class="msg-assistant">
                    <Markdown content={b().block.content} streaming={generating() && isLastBlock(b().index)} />
                  </div>
                )}
              </Match>
              <Match when={blockOf("thinking")}>
                {(b) => <ThinkingBlock block={b().block} streaming={generating() && isLastBlock(b().index)} />}
              </Match>
              <Match when={blockOf("tool_use")}>
                {(b) => <ToolCard block={b().block} />}
              </Match>
            </Switch>
          );
        }}
      </Index>

      <Show when={showTyping()}>
        <TypingIndicator />
      </Show>

      <Show when={props.session.pendingApproval}>
        {(approval) => <ApprovalCard sessionId={props.session.info.id} approval={approval()} />}
      </Show>

      <Show when={local().usage}>
        {(usage) => (
          <div class="sp-usage">
            {fmtTokens(usage().inputTokens)} in · {fmtTokens(usage().outputTokens)} out
            <Show when={usage().model}> · {usage().model}</Show>
          </div>
        )}
      </Show>
    </div>
  );
}
