import { Show, For, createSignal, createEffect, onMount } from "solid-js";
import { appStore } from "../../stores/app-store";
import { open } from "@tauri-apps/plugin-dialog";
import * as ipc from "../../ipc";
import { ModelSelector } from "./ModelSelector";
import type { Attachment } from "../../types";
import type { SlashCommand } from "../../ipc";

export function Composer() {
  const { store, setStore, sendUserMessage } = appStore;

  const isActive = () => store.activeTab !== null && !store.activeTab.startsWith("__");
  const isGenerating = () => {
    if (!store.activeTab) return false;
    const s = store.sessionStatuses[store.activeTab];
    return s === "generating" || s === "interrupting";
  };

  const isInterrupting = () => {
    if (!store.activeTab) return false;
    return store.sessionStatuses[store.activeTab] === "interrupting";
  };

  const sessionStatus = () => {
    if (!store.activeTab) return null;
    return store.sessionStatuses[store.activeTab] || null;
  };

  const folderLabel = () => {
    if (!store.activeTab) return "No folder";
    const project = store.projects.find((p) =>
      p.threads.some((t) => t.id === store.activeTab)
    );
    if (!project || project.path === ".") return "No folder";
    return project.path.split("/").pop() || project.path;
  };

  async function pickFolder() {
    const selected = await open({ directory: true, title: "Select project folder" });
    if (!selected || !store.activeTab) return;
    const path = selected as string;

    const existing = store.projects.find((p) => p.path === path);
    if (existing) {
      await ipc.moveThreadToProject(store.activeTab, existing.id);
      moveThreadLocally(store.activeTab, existing.id);
    } else {
      const dirName = path.split("/").pop() || path;
      const created = await ipc.createProject(dirName, path);
      setStore("projects", (prev) => [
        ...prev,
        { ...created, color: null, collapsed: false, threads: [] },
      ]);
      await ipc.moveThreadToProject(store.activeTab!, created.id);
      moveThreadLocally(store.activeTab!, created.id);
    }
  }

  function moveThreadLocally(threadId: string, targetProjectId: string) {
    setStore("projects", (projects) => {
      const thread = projects.flatMap((p) => p.threads).find((t) => t.id === threadId);
      if (!thread) return projects;
      return projects.map((p) => ({
        ...p,
        threads:
          p.id === targetProjectId
            ? [...p.threads.filter((t) => t.id !== threadId), thread]
            : p.threads.filter((t) => t.id !== threadId),
      }));
    });
  }

  // Slash command autocomplete
  const [slashCommands, setSlashCommands] = createSignal<SlashCommand[]>([]);
  const [showSlashMenu, setShowSlashMenu] = createSignal(false);
  const [slashFilter, setSlashFilter] = createSignal("");
  const [slashIndex, setSlashIndex] = createSignal(0);

  // Load slash commands from IPC + merge SDK init commands from store
  createEffect(() => {
    const p = store.selectedProvider;
    ipc.listSlashCommands(p).then(setSlashCommands).catch(() => setSlashCommands([]));
  });

  const allSlashCommands = () => {
    const ipcCmds = slashCommands();
    const sdkCmds = store.availableSlashCommands || [];
    // Merge SDK commands that aren't already in the IPC list
    const existing = new Set(ipcCmds.map((c) => c.name));
    const merged = [...ipcCmds];
    for (const cmd of sdkCmds) {
      const name = cmd.startsWith("/") ? cmd : `/${cmd}`;
      if (!existing.has(name)) {
        merged.push({ name, description: `Skill: ${cmd}`, source: "sdk" });
      }
    }
    return merged;
  };

  const filteredSlash = () => {
    const q = slashFilter().toLowerCase();
    const cmds = allSlashCommands();
    if (!q) return cmds.slice(0, 15);
    return cmds.filter((c) => c.name.toLowerCase().includes(q) || c.description.toLowerCase().includes(q)).slice(0, 15);
  };

  function handleSlashSelect(cmd: SlashCommand) {
    setStore("composerText", cmd.name + " ");
    setShowSlashMenu(false);
  }

  async function handleStop() {
    if (!store.activeTab) return;
    const threadId = store.activeTab;
    const status = store.sessionStatuses[threadId];

    if (status === "interrupting") {
      // Second click: force kill
      try {
        await ipc.stopSession(threadId);
        appStore.setStore("sessionStatuses", threadId, "ready");
      } catch (e) {
        console.error("Failed to kill session:", e);
      }
      return;
    }

    // First click: graceful interrupt via SIGINT
    try {
      setStore("sessionStatuses", threadId, "interrupting");
      await ipc.interruptSession(threadId);

      // After 2 seconds, if still interrupting, the button will show "Force stop"
      // (the status stays "interrupting" until a turn_completed/turn_aborted event
      // resets it to "ready", or the user clicks again to force kill)
    } catch (e) {
      console.error("Failed to interrupt session:", e);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (showSlashMenu()) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSlashIndex((i) => Math.min(i + 1, filteredSlash().length - 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSlashIndex((i) => Math.max(i - 1, 0));
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        const cmds = filteredSlash();
        if (cmds.length > 0) handleSlashSelect(cmds[slashIndex()]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setShowSlashMenu(false);
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendUserMessage();
    }
  }

  const [dragOver, setDragOver] = createSignal(false);

  function removeAttachment(id: string) {
    setStore("attachments", (a) => a.filter((x) => x.id !== id));
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    setDragOver(false);
    const files = e.dataTransfer?.files;
    if (!files) return;
    for (const file of Array.from(files)) {
      const reader = new FileReader();
      reader.onload = () => {
        const content = reader.result as string;
        const ext = file.name.split(".").pop() || "";
        const lang = { ts: "typescript", tsx: "tsx", js: "javascript", jsx: "jsx", py: "python", rs: "rust", go: "go", css: "css", html: "html", json: "json", md: "markdown", yaml: "yaml", yml: "yaml", toml: "toml", sh: "bash" }[ext] || "";
        setStore("attachments", (prev) => [...prev, {
          id: crypto.randomUUID(),
          type: "file" as const,
          name: file.name,
          content,
          language: lang,
        }]);
      };
      reader.readAsText(file);
    }
  }

  const providerLabel = () =>
    store.selectedProvider === "claude_code" ? "Claude Code" : "Codex";

  const statusColor = () => {
    const s = sessionStatus();
    if (s === "ready") return "var(--green)";
    if (s === "generating") return "var(--sky)";
    if (s === "interrupting") return "var(--amber)";
    if (s === "starting") return "var(--amber)";
    if (s === "error") return "var(--red)";
    return null;
  };

  const statusLabel = () => {
    const s = sessionStatus();
    if (s === "ready") return "Ready";
    if (s === "generating") return "Working";
    if (s === "interrupting") return "Interrupting...";
    if (s === "starting") return "Connecting";
    if (s === "error") return "Error";
    return null;
  };

  return (
    <Show when={isActive()}>
      <div class="composer-wrapper">
        <div
          class="composer-card"
          classList={{ "drag-over": dragOver() }}
          onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
          onDragLeave={() => setDragOver(false)}
          onDrop={handleDrop}
        >
          <Show when={store.attachments.length > 0}>
            <div class="attachment-chips">
              <For each={store.attachments}>
                {(att) => (
                  <div class="attachment-chip" classList={{ extraction: att.type === "extraction" }}>
                    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      {att.type === "extraction"
                        ? <><path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 013 3L7 19l-4 1 1-4z"/></>
                        : <><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/></>
                      }
                    </svg>
                    <span class="attachment-name">{att.name}</span>
                    <button class="attachment-remove" onClick={() => removeAttachment(att.id)}>
                      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                    </button>
                  </div>
                )}
              </For>
            </div>
          </Show>
          {/* Slash command autocomplete */}
          <Show when={showSlashMenu() && filteredSlash().length > 0}>
            <div class="slash-menu">
              <For each={filteredSlash()}>
                {(cmd, idx) => (
                  <button
                    class="slash-item"
                    classList={{ "slash-item--active": idx() === slashIndex() }}
                    onMouseDown={(e) => { e.preventDefault(); handleSlashSelect(cmd); }}
                    onMouseEnter={() => setSlashIndex(idx())}
                  >
                    <span class="slash-name">{cmd.name}</span>
                    <span class="slash-desc">{cmd.description}</span>
                    <Show when={cmd.source !== "built-in"}>
                      <span class="slash-source">{cmd.source}</span>
                    </Show>
                  </button>
                )}
              </For>
            </div>
          </Show>
          <div class="composer-input-row">
            <textarea
              class="composer-input"
              placeholder={isGenerating() ? "Send a message to steer…" : "Message..."}
              value={store.composerText}
              onInput={(e) => {
                const val = e.currentTarget.value;
                setStore("composerText", val);
                const el = e.currentTarget;
                el.style.height = "auto";
                el.style.height = Math.min(el.scrollHeight, 120) + "px";

                // Slash command detection
                if (val.startsWith("/") && !val.includes(" ")) {
                  setSlashFilter(val);
                  setShowSlashMenu(true);
                  setSlashIndex(0);
                } else {
                  setShowSlashMenu(false);
                }
              }}
              onKeyDown={handleKeyDown}
              rows={1}
            />
            <input
              type="file"
              multiple
              id="attach-file-input"
              style="display:none;"
              onChange={(e) => {
                const files = e.currentTarget.files;
                if (!files) return;
                for (const file of Array.from(files)) {
                  const reader = new FileReader();
                  reader.onload = () => {
                    const content = reader.result as string;
                    const ext = file.name.split(".").pop() || "";
                    const lang = { ts: "typescript", tsx: "tsx", js: "javascript", jsx: "jsx", py: "python", rs: "rust", go: "go", css: "css", html: "html", json: "json", md: "markdown" }[ext] || "";
                    setStore("attachments", (prev) => [...prev, {
                      id: crypto.randomUUID(),
                      type: "file" as const,
                      name: file.name,
                      content,
                      language: lang,
                    }]);
                  };
                  reader.readAsText(file);
                }
                e.currentTarget.value = "";
              }}
            />
            <button
              class="attach-btn"
              onClick={() => document.getElementById("attach-file-input")?.click()}
              title="Attach files"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"/>
              </svg>
            </button>
            <Show when={isGenerating() && !store.composerText.trim()}>
              <button
                class="send-btn stop"
                classList={{ "force-stop": isInterrupting() }}
                onClick={handleStop}
                title={isInterrupting() ? "Force stop" : "Interrupt"}
              >
                <Show when={isInterrupting()} fallback={
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" rx="2" /></svg>
                }>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                </Show>
              </button>
            </Show>
            <Show when={!isGenerating() || store.composerText.trim()}>
              <button
                class="send-btn"
                classList={{ steering: isGenerating() && !!store.composerText.trim() }}
                onClick={sendUserMessage}
                title={isGenerating() ? "Send to steer response" : "Send message"}
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="12" y1="19" x2="12" y2="5" /><polyline points="5 12 12 5 19 12" />
                </svg>
              </button>
            </Show>
          </div>
          <div class="composer-meta">
            <button
              class="meta-pill"
              onClick={() => setStore("providerPickerOpen", true)}
              title="Switch between Claude Code and Codex"
            >
              {providerLabel()}
              <span class="provider-dot" style={{ background: "var(--green)" }} />
              <svg class="chevron" width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><polyline points="6 9 12 15 18 9" /></svg>
            </button>
            <ModelSelector />
            <button
              class="meta-pill"
              classList={{ "meta-pill--active": store.autoAcceptEnabled }}
              onClick={() => {
                const next = !store.autoAcceptEnabled;
                appStore.setStore("autoAcceptEnabled", next);
                ipc.setSetting("permission_mode", next ? "bypassPermissions" : "default").catch(() => {});
                appStore.persistState();
              }}
              title={store.autoAcceptEnabled ? "Toggle auto-accept for tool permissions — currently auto-accepting all tools (click to require approval)" : "Toggle auto-accept for tool permissions — currently requiring approval (click to auto-accept)"}
            >
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
              </svg>
              {store.autoAcceptEnabled ? "Auto" : "Ask"}
            </button>
            <button class="meta-pill subtle" onClick={pickFolder} title="Set the working directory for this thread">
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
              </svg>
              {folderLabel()}
            </button>
            <div class="spacer" />
            <Show when={statusLabel()}>
              <div class="status-pill">
                <span class="status-dot" style={{ background: statusColor()! }} />
                <span class="status-text" style={{ color: statusColor()! }}>
                  {statusLabel()}
                </span>
              </div>
            </Show>
          </div>
        </div>
      </div>
    </Show>
  );
}

if (!document.getElementById("composer-styles")) {
  const style = document.createElement("style");
  style.id = "composer-styles";
  style.textContent = `
    .composer-wrapper {
      padding: 8px 20px 16px;
      display: flex;
      justify-content: center;
      flex-shrink: 0;
    }
    .composer-card {
      width: 100%;
      max-width: 768px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-xl);
      padding: 12px 14px;
      display: flex;
      flex-direction: column;
      gap: 8px;
      transition: border-color 0.2s, box-shadow 0.2s;
      position: relative;
    }
    .composer-card:focus-within {
      border-color: var(--border-glow);
      box-shadow: 0 0 0 2px var(--primary-glow), 0 4px 16px rgba(0, 0, 0, 0.15);
    }
    .composer-card.drag-over {
      border-color: var(--primary);
      background: rgba(107, 124, 255, 0.04);
    }
    .attachment-chips {
      display: flex;
      flex-wrap: wrap;
      gap: 4px;
      padding-bottom: 4px;
    }
    .attachment-chip {
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 3px 6px 3px 8px;
      background: var(--bg-muted);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      font-size: 11px;
      color: var(--text-secondary);
    }
    .attachment-chip.extraction {
      border-color: rgba(107, 124, 255, 0.2);
      background: rgba(107, 124, 255, 0.06);
      color: var(--primary);
    }
    .attachment-chip svg { flex-shrink: 0; }
    .attachment-name {
      max-width: 120px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .attachment-remove {
      width: 16px; height: 16px;
      display: flex; align-items: center; justify-content: center;
      border-radius: 3px;
      color: var(--text-tertiary);
      transition: background 0.1s, color 0.1s;
    }
    .attachment-remove:hover { background: var(--bg-accent); color: var(--text); }
    .composer-input-row {
      display: flex;
      align-items: flex-end;
      gap: 8px;
    }
    .composer-input {
      flex: 1;
      background: none;
      border: none;
      color: var(--text);
      font-size: 14px;
      resize: none;
      outline: none;
      padding: 4px 0;
      line-height: 1.4;
      min-height: 22px;
      max-height: 120px;
      font-family: var(--font-body);
    }
    .attach-btn {
      width: 32px;
      height: 32px;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-tertiary);
      flex-shrink: 0;
      transition: color 0.12s, background 0.12s;
    }
    .attach-btn:hover { color: var(--text-secondary); background: var(--bg-accent); }
    .send-btn {
      width: 32px;
      height: 32px;
      border-radius: 50%;
      background: var(--primary);
      color: white;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
      transition: background 0.15s, transform 0.1s;
    }
    .send-btn:hover { filter: brightness(1.1); transform: scale(1.04); }
    .send-btn:active { transform: scale(0.96); }
    .send-btn.stop { background: var(--amber, #e6b84d); }
    .send-btn.stop.force-stop { background: var(--red); }
    .send-btn.steering {
      background: var(--amber, #e6b84d);
    }
    /* ── Slash command menu ── */
    @keyframes slash-menu-in {
      from { opacity: 0; transform: translateY(6px) scale(0.98); }
      to { opacity: 1; transform: translateY(0) scale(1); }
    }
    .slash-menu {
      position: absolute;
      bottom: 100%;
      left: 0;
      right: 0;
      margin-bottom: 4px;
      background: var(--bg-card);
      border: 1px solid var(--border-strong);
      border-radius: var(--radius-md);
      box-shadow: 0 -8px 32px rgba(0, 0, 0, 0.4), 0 0 1px rgba(255,255,255,0.05);
      max-height: 280px;
      overflow-y: auto;
      padding: 4px;
      z-index: 50;
      transform-origin: bottom center;
      animation: slash-menu-in 120ms cubic-bezier(0.16, 1, 0.3, 1) both;
    }
    .slash-item {
      display: flex;
      align-items: center;
      gap: 8px;
      width: 100%;
      padding: 7px 10px;
      border-radius: var(--radius-sm);
      font-size: 12px;
      transition: background 0.08s;
      text-align: left;
    }
    .slash-item:hover, .slash-item--active {
      background: var(--bg-accent);
    }
    .slash-name {
      font-weight: 600;
      color: var(--primary);
      font-family: var(--font-mono);
      font-size: 12px;
      white-space: nowrap;
    }
    .slash-desc {
      flex: 1;
      color: var(--text-secondary);
      font-size: 11px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .slash-source {
      font-size: 9px;
      font-weight: 500;
      color: var(--text-tertiary);
      padding: 1px 5px;
      background: var(--bg-muted);
      border-radius: var(--radius-pill);
      white-space: nowrap;
    }
    .composer-meta {
      display: flex;
      align-items: center;
      gap: 6px;
    }
    .meta-pill {
      font-size: 11px;
      font-weight: 500;
      color: var(--text-secondary);
      padding: 4px 10px 4px 8px;
      border-radius: var(--radius-pill);
      background: var(--bg-muted);
      border: 1px solid var(--border);
      transition: background 0.15s, border-color 0.15s;
      display: flex;
      align-items: center;
      gap: 5px;
    }
    .meta-pill:hover { background: var(--bg-accent); border-color: var(--border-strong); }
    .meta-pill--active {
      background: rgba(76, 214, 148, 0.1);
      border-color: rgba(76, 214, 148, 0.2);
      color: var(--green);
    }
    .meta-pill--active:hover {
      background: rgba(76, 214, 148, 0.15);
      border-color: rgba(76, 214, 148, 0.3);
    }
    .meta-pill .chevron {
      color: var(--text-tertiary);
      transition: color 0.15s;
    }
    .meta-pill:hover .chevron { color: var(--text-secondary); }
    .provider-dot {
      width: 5px;
      height: 5px;
      border-radius: 50%;
      flex-shrink: 0;
    }
    .meta-pill.subtle {
      background: none;
      border-color: transparent;
      color: var(--text-tertiary);
    }
    .meta-pill.subtle:hover {
      background: var(--bg-hover);
      border-color: var(--border);
    }
    .spacer { flex: 1; }
    .status-pill {
      display: flex;
      align-items: center;
      gap: 5px;
      padding: 3px 8px;
      border-radius: var(--radius-pill);
      background: var(--bg-muted);
    }
    .status-dot {
      width: 5px;
      height: 5px;
      border-radius: 50%;
    }
    .status-text { font-size: 10px; font-weight: 500; }
  `;
  document.head.appendChild(style);
}
