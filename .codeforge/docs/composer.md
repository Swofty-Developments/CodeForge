The changes in `app-store.ts` are entirely about index staleness detection (the `handleIndexStatus`, `staleModal`, `indexStatusByPath` fields and related functions). The `composerPrefill` field is unchanged at line 75, and none of the edits touch the Composer feature.

---
# Composer

## Purpose

The session composer is an autosize textarea that accepts user prompts, slash commands, and file attachments. It handles keyboard shortcuts (Cmd+Enter to send, Escape to dismiss slash menu), auto-focus on prefill, and file-drop attachment.

## How it works

- **Auto-resizing textarea** — grows vertically on input up to `MAX_COMPOSER_PX` (168px, ~8 lines) by measuring `scrollHeight` and capping the height.
- **Slash-command popover** — opens when input matches `/\S*` (slash + non-space characters with no trailing space). Arrow keys navigate, Enter/Tab pick, Escape dismisses. Once a space is typed the command is chosen and the menu closes.
- **Attachments** — paperclip button opens a file picker; drag-drop is handled via Tauri's `onDragDropEvent` with physical-position hit-testing against the card bounds to accept drops only over the composer. Attachments render as chips above the textarea.
- **Prefill from "Ask Claude" flow** — watches `appStore.composerPrefill`; when set, seeds the textarea, clears the prefill, and auto-focuses.
- **Send** — Enter (no shift) or clicking the send button calls `withAttachments(text, paths)` to format the prompt, clears input/attachments, and invokes `props.onSend`. While generating, the send button becomes a stop button.
- **Permission mode switcher** — lives in the meta row alongside attach/send; delegates to `ModeControl`.

## Key files

- **`Composer.tsx`** — main component; owns input state, slash-menu lifecycle, attachment picker + drag-drop, auto-resize logic, and send flow.
- **`SlashMenu.tsx`** — presentational popover component; filters + ranks commands (prefix matches above substring matches), renders item list.
- **`styles-composer.ts`** — CSS module for composer card, attachment chips, slash menu, drop hint overlay, and send/attach buttons.

## Invariants & gotchas

- **Slash menu closes on space** — the pattern `^\/(\S*)$` means the menu is only open while the entire input is `/something-with-no-spaces`. Typing a space finalizes the command name and dismisses the menu. The `slashDismissed` flag prevents re-opening until the input changes again.
- **Enter is always send when slash menu is closed** — Enter submits the form unless the slash menu is open (where it picks the selected command) or Shift is held (newline). This means slash commands must be picked via Enter/Tab/click before the user can type arguments on a newline.
- **Drag-drop hit-testing is physical pixels** — Tauri delivers drop positions in physical coordinates; the `overCard` check divides by `devicePixelRatio` before comparing against the card's CSS `getBoundingClientRect()`. If this logic is wrong, drops land on the wrong target or are ignored.
- **Resize must run after every input change and prefill** — the auto-height logic resets `textarea.style.height = "auto"` then measures `scrollHeight`. If `resize()` is skipped after setting `value` (e.g., prefill or slash-pick), the textarea won't grow to fit the new content.
- **Attachments are deduplicated by Set** — `addPaths` merges new paths into a Set to prevent duplicates. Removing an attachment is linear-filter over the array; fine for small counts but not optimized for hundreds of files.
