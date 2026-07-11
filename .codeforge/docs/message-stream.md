The changes described in the notes affect the **indexing staleness detection** backend and frontend, not the Message Stream feature. The Message Stream renders session messages, tool cards, thinking blocks, and markdown — it has nothing to do with index status polling or stale modals.

The edited files confirm this:
- `staleness.rs`, `state.rs`, `repo_open.rs` — backend staleness polling
- `app-store.ts` — `handleIndexStatus` logic, stale modal state
- `ipc.ts` — IPC type changes for IndexStatus

None of these touch `MessageStream.tsx`, `blocks.tsx`, `markdown.tsx`, or the message rendering logic.

---
# Message Stream

## Purpose

Renders the session message list with user bubbles, assistant text blocks, tool-use cards, thinking blocks, and system notifications. Auto-scrolls to bottom while streaming, defers scroll when user scrolls away.

## How it works

- Messages render straight from `session.messages` (single source of truth): user bubbles right-aligned, assistant blocks full-width, system pills centered.
- Scroll anchoring: 150px away-threshold, 80ms debounce. Tracks any content growth (blocks, tool outputs, token counts) and schedules a follow-scroll unless the user has scrolled away.
- Streaming flag (`runState === "generating"`) controls the typing indicator visibility and per-block streaming state; the trailing assistant text block gets a streaming fade-up animation.
- Tool cards and thinking blocks collapse/expand inline; chevron rotates, body uses `grid-template-rows: 0fr → 1fr` for smooth height transition.
- Markdown rendering via `marked` + `highlight.js`, code blocks with lang label and hover copy button; styles injected once with an id guard.
- Approval requests render as amber-tinted cards with Approve/Deny buttons that call `appStore.approveRequest`.

## Key files

- `MessageStream.tsx` — scroll container, message list loop, scroll anchoring logic, usage footer
- `blocks.tsx` — `ToolCard`, `ThinkingBlock`, `TypingIndicator`, `ApprovalCard` components
- `markdown.tsx` — `Markdown` component: `marked` config, `hljs` highlighting, copy-button delegation, streaming class toggle
- `styles-stream.ts` — CSS for user/assistant/system messages, tool cards, thinking blocks, typing shimmer, approval card animations

## Invariants & gotchas

- **Scroll anchoring must track all content growth**: block content, tool outputs, tool status, usage tokens. Missing a dependency means scroll won't follow.
- **Trailing block detection**: `mi === lastMsgIndex() && bi === blocks.length - 1` determines streaming state; off-by-one here breaks the streaming fade.
- **Typing indicator exclusion**: shown only when generating AND (no messages OR last message is not assistant OR last block is not text). If logic is wrong, typing and streaming text appear simultaneously.
- **Markdown styles inject once**: the id guard `md-styles` prevents duplicate `<style>` tags; removing it causes style bloat on every message render.
- **Tool input is partial JSON while streaming**: `toolSummary` must catch parse errors and show raw text instead of crashing.
- **Copy buttons live in `innerHTML`**: event delegation via `onClick` on `.md-render` is required; direct event handlers won't work.
- **Approval card does not clear `pendingApproval`**: `appStore.approveRequest` does that; local state mutation here would desync from Rust backend.
