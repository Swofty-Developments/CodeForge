# Living Doc Refresh

## Purpose

Auto-maintains `.codeforge/docs/<slug>.md` living docs by invoking headless `claude` after agent turns complete. When a Stop hook fires (`SessionEnded` event), refreshes the doc for every feature whose files were edited, folding in the turn's edits, commands, and agent-recorded notes.

## How it works

- **Trigger**: daemon listens for `SessionEnded` timeline events (Stop hook fired).
- **Turn slicing**: fetches recent timeline events (up to 1000), filters to the session, extracts file edits, commands, and notes before the end event.
- **Refresh queue**: for each touched feature, queues a serialized doc refresh (semaphore of 1; slugs already queued are skipped to avoid duplicate work).
- **Headless claude**: spawns `claude -p --output-format json` with a prompt containing the feature metadata, existing doc verbatim (or fresh-doc state if missing), and rendered turn events; reads the returned markdown.
- **Write-back**: strips fences, clamps to 60 lines, writes atomically via tmp+rename to `.codeforge/docs/<slug>.md`.
- **Pinned features skipped**: if `feature.pinned` is true, refresh is skipped — human edits survive.

## Key files

- **crates/forge-index/src/refresh.rs** — `refresh_feature_doc()` entry point, prompt builder, turn-event rendering logic.
- **crates/forge-index/src/headless.rs** — spawns `claude` with read-only tools, pipes prompt on stdin, parses JSON envelope, enforces 6-minute timeout.
- **crates/forge-daemon/src/doc_refresh.rs** — daemon worker listening for `SessionEnded` events, turn slicing, queue-deduping, outcome recording.
- **crates/forge-index/src/prompt.rs** — defines `DOC_MAX_LINES` (60), `doc_prompt()` for cold-start docs, `role_label()` helper.

## Invariants & gotchas

- **Pinned check is load-bearing**: `feature.pinned` must be checked BEFORE calling `refresh_feature_doc()` or human-written docs will be overwritten.
- **Turn slice must be filtered by session**: using unfiltered recent events would blend multiple sessions' work into one doc update.
- **Atomic write required**: tmp+rename is the only safe write pattern — a half-written doc mid-refresh is visible to readers without it.
- **Timeout must kill child**: `cmd.kill_on_drop(true)` and the timeout-elapsed branch dropping the future together prevent zombie `claude` processes.
- **Model is hardcoded**: `MODEL = "claude-sonnet-4-5"` in headless.rs; changing it requires a code edit, not config.
- **Edit count deduping**: `render_events()` counts edits per file and dedupes commands so the prompt doesn't balloon on repeated actions.
