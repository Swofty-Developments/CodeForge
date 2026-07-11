# Message Stream

**Purpose**

Renders the session's message history: user prompts, assistant text/thinking, tool use cards, approval requests, and session lifecycle events. Auto-scrolls to bottom, supports markdown rendering, and handles code blocks with syntax highlighting.

**How it works**

- Renders straight from `session.messages` (single source of truth): user bubbles, system pills, and assistant messages whose `ContentBlock`s render as text / thinking / tool cards.
- Scroll anchoring with a 150px away-threshold and 80ms debounce: tracks `scrolledAway` state via `onScroll`; if the user hasn't scrolled away, `scheduleScroll` auto-follows the bottom on every content change (new messages, streaming text, tool output updates, usage increments).
- Switching sessions or adding a message resets `scrolledAway` to false and snaps to the bottom immediately.
- Typing indicator appears when `runState === "generating"` and the last assistant block is not a live text stream (e.g., user just sent, or a tool is running).
- Tool cards, thinking blocks, approval/question cards, and the usage footer all render inline via `<Show>` guards; markdown rendering (via `marked` + `highlight.js`) lives in `Markdown.tsx` with injected global styles and delegated copy-button handlers.

**Key files**

- `MessageStream.tsx` — scroll-anchored stream container, message role dispatch, auto-scroll logic, and usage footer
- `blocks.tsx` — `ToolCard`, `ThinkingBlock`, `TypingIndicator`, `ApprovalCard`, `QuestionCard` components with expand/collapse state and status tints
- `markdown.tsx` — `Markdown` component using `marked` + `hljs` for syntax highlighting, code block lang labels, hover copy buttons, and streaming last-line fade-up
- `styles-stream.ts` — CSS for the stream, user/assistant/system messages, tool cards, thinking blocks, typing indicator, approval/question cards, and usage footer

**Invariants & gotchas**

- `session.messages` is the single source of truth — never mutate it outside the store; the stream renders reactively from it.
- Scroll anchoring depends on tracking every content change: `runState`, `pendingApproval`, `pendingQuestion`, `messages[].blocks[].content`, `blocks[].toolOutput`, `blocks[].toolStatus`, and `usage.outputTokens`. Missing a dependency means the stream won't follow a streamed change.
- `scrolledAway` must reset on session switch (via `props.session.info.id`) and message add (via `messages.length`) so the user isn't stuck scrolled-away in a new conversation.
- Typing indicator logic: show when `generating()` and the last assistant message either has no blocks or the trailing block is not `type: "text"` — otherwise the live text is already streaming inline.
- Markdown styles are injected once (id guard) since `Markdown` renders per message; copy buttons live inside `innerHTML` and are handled by delegation (`onRenderClick`).
- Tool card output is truncated at 4000 chars with a "… (N more chars)" suffix to prevent DOM bloat on huge outputs.
- Approval/question cards optimistically clear `pendingApproval` / `pendingQuestion` on submit; the store action clears it again defensively.
