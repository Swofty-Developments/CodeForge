/* The session composer (FZ-4): autosize textarea + slash-command popover +
 * file-attachment chips (paperclip picker and drag-drop onto the card). Owns all
 * compose-time state; hands the finished prompt to `onSend`. The interactive
 * permission-mode switch (ModeControl) and send/stop live in the meta row. */

import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { appStore } from "../../stores/app-store";
import { savePastedImage } from "../../ipc";
import { ModeControl } from "./ModeControl";
import { SlashMenu, filterSlashCommands } from "./SlashMenu";
import { basename, withAttachments } from "./attachments";
import type { PermissionMode, SessionUi } from "../../types";

const MAX_COMPOSER_PX = 168; // ~8 lines at 13.5px / 1.45

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve((reader.result as string).split(",", 2)[1] ?? "");
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

export function Composer(props: {
  disabled: boolean;
  session: SessionUi | null;
  currentMode: PermissionMode;
  onSelectMode: (mode: PermissionMode) => void;
  onSend: (text: string) => Promise<void>;
  onStop: () => void;
}) {
  const [input, setInput] = createSignal("");
  const [attachments, setAttachments] = createSignal<string[]>([]);
  const [slashSel, setSlashSel] = createSignal(0);
  const [slashDismissed, setSlashDismissed] = createSignal(false);
  const [dragOver, setDragOver] = createSignal(false);
  let taRef: HTMLTextAreaElement | undefined;
  let cardRef: HTMLDivElement | undefined;

  const generating = () => props.session?.runState === "generating";
  const canSend = () => !props.disabled && !!input().trim();

  function resize(): void {
    if (!taRef) return;
    taRef.style.height = "auto";
    taRef.style.height = `${Math.min(taRef.scrollHeight, MAX_COMPOSER_PX)}px`;
  }

  // ── Slash-command popover ───────────────────────────────────────────────
  // Open only while the whole input is one "/token" (no space yet = still naming
  // the command). Once the user types a space the command is chosen and we close.
  const slashFragment = (): string | null => {
    const m = input().match(/^\/(\S*)$/);
    return m ? m[1] : null;
  };
  const slashItems = createMemo(() => {
    const frag = slashFragment();
    if (frag === null) return [];
    return filterSlashCommands(props.session?.slashCommands ?? [], frag);
  });
  const slashOpen = () =>
    !slashDismissed() && slashFragment() !== null && (props.session?.slashCommands.length ?? 0) > 0;

  // Keep the highlighted row valid as the filtered list shrinks/grows.
  createEffect(() => {
    const n = slashItems().length;
    if (slashSel() >= n) setSlashSel(0);
  });

  function pickSlash(name: string): void {
    setInput(`/${name} `);
    setSlashDismissed(true); // a full name is chosen; don't re-open until edited
    queueMicrotask(() => {
      if (!taRef) return;
      taRef.focus();
      const end = taRef.value.length;
      taRef.setSelectionRange(end, end);
      resize();
    });
  }

  function onInput(value: string): void {
    setInput(value);
    setSlashDismissed(false);
    resize();
  }

  function onKeyDown(e: KeyboardEvent): void {
    if (slashOpen() && slashItems().length > 0) {
      const n = slashItems().length;
      if (e.key === "ArrowDown") { e.preventDefault(); setSlashSel((s) => (s + 1) % n); return; }
      if (e.key === "ArrowUp") { e.preventDefault(); setSlashSel((s) => (s - 1 + n) % n); return; }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        pickSlash(slashItems()[slashSel()]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation(); // don't let the app-global Escape act while we own it
        setSlashDismissed(true);
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); void send(); }
  }

  // ── Attachments ─────────────────────────────────────────────────────────
  function addPaths(paths: readonly string[]): void {
    if (paths.length === 0) return;
    setAttachments((prev) => {
      const set = new Set(prev);
      for (const p of paths) set.add(p);
      return [...set];
    });
  }
  function removePath(path: string): void {
    setAttachments((prev) => prev.filter((p) => p !== path));
  }

  // Pasted images have no path, so they are written to a temp file backend-side
  // and attached as a path like any picked file. Text pastes pass through.
  function onPaste(e: ClipboardEvent): void {
    const images = Array.from(e.clipboardData?.items ?? []).filter((it) =>
      it.type.startsWith("image/")
    );
    if (images.length === 0 || props.disabled) return;
    e.preventDefault();
    for (const item of images) {
      const blob = item.getAsFile();
      if (!blob) continue;
      const mime = item.type;
      void blobToBase64(blob)
        .then((data) => savePastedImage(data, mime))
        .then((path) => addPaths([path]))
        .catch((err) => appStore.pushError(`Pasted image failed to attach: ${String(err)}`));
    }
  }

  async function pickFiles(): Promise<void> {
    if (props.disabled) return;
    const res = await open({ multiple: true });
    if (res == null) return;
    addPaths(Array.isArray(res) ? res : [res]);
    queueMicrotask(() => taRef?.focus());
  }

  // Tauri delivers OS file drops as a webview event (not an HTML drop); hit-test
  // its physical position against the card so only drops on the composer attach.
  function overCard(pos: { x: number; y: number }): boolean {
    if (!cardRef) return false;
    const dpr = window.devicePixelRatio || 1;
    const x = pos.x / dpr;
    const y = pos.y / dpr;
    const r = cardRef.getBoundingClientRect();
    return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
  }

  onMount(() => {
    if (!("__TAURI_INTERNALS__" in window)) return; // drag-drop is a Tauri capability
    const pending = getCurrentWebview().onDragDropEvent((ev) => {
      const p = ev.payload;
      if (p.type === "leave") { setDragOver(false); return; }
      if (p.type === "enter" || p.type === "over") { setDragOver(overCard(p.position)); return; }
      if (p.type === "drop") {
        if (overCard(p.position)) addPaths(p.paths);
        setDragOver(false);
      }
    });
    onCleanup(() => { void pending.then((un) => un()); });
  });

  // "Ask Claude about this feature" seeds the composer + focuses it.
  createEffect(() => {
    const pf = appStore.store.composerPrefill;
    if (pf == null) return;
    setInput(pf);
    appStore.clearComposerPrefill();
    queueMicrotask(() => { if (taRef) { taRef.focus(); resize(); } });
  });

  async function send(): Promise<void> {
    const text = input().trim();
    if (!text || props.disabled) return;
    const prompt = withAttachments(text, attachments());
    setInput("");
    setAttachments([]);
    setSlashDismissed(false);
    resize();
    await props.onSend(prompt);
  }

  return (
    <div class="sp-composer-wrap">
      <div
        ref={cardRef}
        class="sp-composer-card"
        classList={{ generating: generating(), disabled: props.disabled, "drag-over": dragOver() }}
      >
        <Show when={attachments().length > 0}>
          <div class="sp-attachments">
            <For each={attachments()}>
              {(path) => (
                <span class="sp-chip" title={path}>
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 12.8l-8.5 8.5a5 5 0 0 1-7-7l8.5-8.5a3.3 3.3 0 0 1 4.7 4.7l-8.5 8.5a1.7 1.7 0 0 1-2.4-2.4l7.9-7.8" />
                  </svg>
                  <span class="sp-chip-name">{basename(path)}</span>
                  <button class="sp-chip-x" title="Remove" onClick={() => removePath(path)}>
                    <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6">
                      <path d="M18 6L6 18M6 6l12 12" />
                    </svg>
                  </button>
                </span>
              )}
            </For>
          </div>
        </Show>

        <div class="sp-input-row">
          <textarea
            ref={taRef}
            class="sp-input"
            rows={1}
            placeholder={props.disabled ? "Open a repository first" : "Ask Claude — type / for commands"}
            disabled={props.disabled}
            value={input()}
            onInput={(e) => onInput(e.currentTarget.value)}
            onKeyDown={onKeyDown}
            onPaste={onPaste}
          />
          <Show when={slashOpen()}>
            <SlashMenu items={slashItems()} selected={slashSel()} onPick={pickSlash} onHover={setSlashSel} />
          </Show>
        </div>

        <div class="sp-composer-meta">
          <ModeControl mode={props.currentMode} disabled={props.disabled} onSelect={props.onSelectMode} />
          <button class="sp-attach-btn" title="Attach files" disabled={props.disabled} onClick={() => void pickFiles()}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12.8l-8.5 8.5a5 5 0 0 1-7-7l8.5-8.5a3.3 3.3 0 0 1 4.7 4.7l-8.5 8.5a1.7 1.7 0 0 1-2.4-2.4l7.9-7.8" />
            </svg>
          </button>
          <div class="sp-composer-spacer" />
          <Show
            when={generating()}
            fallback={
              <button class="sp-send" title="Send (Enter)" disabled={!canSend()} onClick={() => void send()}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
                  <path d="M12 19V5M5 12l7-7 7 7" />
                </svg>
              </button>
            }
          >
            <button class="sp-send stop" title="Stop response" onClick={() => props.onStop()}>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                <rect x="6" y="6" width="12" height="12" rx="2" />
              </svg>
            </button>
          </Show>
        </div>

        <Show when={dragOver()}>
          <div class="sp-drop-hint">Drop files to attach</div>
        </Show>
      </div>
    </div>
  );
}
