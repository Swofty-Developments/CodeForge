/**
 * Terminal slice (FZ-3) — the bottom terminal panel model: the list of PTY tabs,
 * the active tab, and panel open/height. Each tab's `cwd` is bound at open time
 * to whatever context is active, so terminals are worktree-scoped: switching
 * contexts changes where NEW terminals open, while existing ones keep their cwd.
 *
 * xterm wiring and the terminal:data / terminal:exit listeners live in the panel
 * view (components/terminal/**); this slice owns only the tab bookkeeping.
 */

import { type SetStoreFunction } from "solid-js/store";
import * as ipc from "../ipc";
import type { AppStore } from "./app-store";

const MIN_PANEL_H = 120;

/** Last path segment — the terminal tab label (mirrors the shell's cwd). */
function basename(p: string): string {
  const parts = p.replace(/\/+$/, "").split("/");
  const last = parts[parts.length - 1];
  return last.length > 0 ? last : p;
}

export function createTerminalSlice(
  store: AppStore,
  setStore: SetStoreFunction<AppStore>,
  pushError: (message: string) => void,
) {
  /** Open a PTY rooted at the ACTIVE context path (worktree-scoped) and focus it. */
  async function openTerminal(): Promise<void> {
    const cwd = store.activeContextPath;
    if (!cwd) {
      pushError("open a repository before starting a terminal");
      return;
    }
    try {
      const id = await ipc.openTerminal(cwd);
      setStore("terminals", (ts) => [...ts, { id, title: basename(cwd), cwd, exited: false }]);
      setStore("activeTerminalId", id);
      setStore("terminalPanelOpen", true);
    } catch (e) {
      pushError(String(e));
    }
  }

  /** Drop a tab from the strip and re-point the active tab to a neighbour. */
  function removeTerminal(id: string): void {
    const idx = store.terminals.findIndex((t) => t.id === id);
    const remaining = store.terminals.filter((t) => t.id !== id);
    setStore("terminals", remaining);
    if (store.activeTerminalId === id) {
      const next = remaining[Math.min(idx, remaining.length - 1)];
      setStore("activeTerminalId", next ? next.id : null);
    }
  }

  /** Kill the PTY and remove its tab. */
  async function closeTerminal(id: string): Promise<void> {
    try {
      await ipc.closeTerminal(id);
    } catch (e) {
      pushError(String(e));
    }
    removeTerminal(id);
  }

  /** terminal:exit — keep the tab (so the output stays readable) but mark it dead. */
  function markTerminalExited(id: string): void {
    const i = store.terminals.findIndex((t) => t.id === id);
    if (i >= 0) setStore("terminals", i, "exited", true);
  }

  function setActiveTerminal(id: string): void {
    setStore("activeTerminalId", id);
  }

  function setTerminalPanelHeight(px: number): void {
    const max = Math.max(MIN_PANEL_H, Math.round(window.innerHeight * 0.8));
    setStore("terminalPanelHeight", Math.min(max, Math.max(MIN_PANEL_H, px)));
  }

  /** Cmd+J / status-bar button. Opening an empty panel spawns the first terminal. */
  function toggleTerminalPanel(): void {
    const open = !store.terminalPanelOpen;
    setStore("terminalPanelOpen", open);
    if (open && store.terminals.length === 0 && store.activeContextPath) {
      void openTerminal();
    }
  }

  return {
    openTerminal,
    closeTerminal,
    markTerminalExited,
    setActiveTerminal,
    setTerminalPanelHeight,
    toggleTerminalPanel,
  };
}
