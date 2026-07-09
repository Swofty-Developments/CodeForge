/* One xterm instance bound to a live PTY (FZ-3). Owns the Terminal + FitAddon for
 * its tab id and registers itself with the panel so terminal:data can route to it.
 * onData → write_terminal; ResizeObserver → fit + resize_terminal. The theme is
 * mapped to the Zed One Dark tokens. Instances stack absolutely; only the active
 * one is visible (kept sized via inset:0 so fit stays correct when it returns). */

import { createEffect, onCleanup, onMount } from "solid-js";
import { Terminal, type ITheme } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import * as ipc from "../../ipc";
import type { TerminalTab } from "../../types";

/** Zed One Dark, matching the app's --bg-base / --editor-fg and ANSI palette. */
const ONE_DARK: ITheme = {
  background: "#282c33",
  foreground: "#acb2be",
  cursor: "#74ade8",
  cursorAccent: "#282c33",
  selectionBackground: "rgba(116, 173, 232, 0.30)",
  black: "#282c33",
  red: "#d07277",
  green: "#a1c181",
  yellow: "#dec184",
  blue: "#74ade8",
  magenta: "#b478c0",
  cyan: "#6eb4bf",
  white: "#acb2be",
  brightBlack: "#5d636f",
  brightRed: "#d07277",
  brightGreen: "#a1c181",
  brightYellow: "#dec184",
  brightBlue: "#74ade8",
  brightMagenta: "#cd7ca8",
  brightCyan: "#6eb4bf",
  brightWhite: "#dce0e5",
};

const FONT_STACK = '"JetBrains Mono", "IBM Plex Mono", ui-monospace, "SF Mono", Menlo, monospace';

interface Props {
  tab: TerminalTab;
  active: boolean;
  register: (id: string, term: Terminal) => void;
  unregister: (id: string) => void;
}

export function TerminalInstance(props: Props) {
  let host!: HTMLDivElement;
  let term: Terminal | undefined;
  let fit: FitAddon | undefined;
  let ro: ResizeObserver | undefined;

  function fitAndResize(): void {
    if (!term || !fit) return;
    if (host.clientWidth === 0 || host.clientHeight === 0) return;
    fit.fit();
    void ipc.resizeTerminal(props.tab.id, term.cols, term.rows);
  }

  onMount(() => {
    term = new Terminal({
      theme: ONE_DARK,
      fontFamily: FONT_STACK,
      fontSize: 12,
      lineHeight: 1.25,
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: true,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    fitAndResize();
    term.onData((d) => void ipc.writeTerminal(props.tab.id, d));
    props.register(props.tab.id, term);

    ro = new ResizeObserver(() => fitAndResize());
    ro.observe(host);
  });

  // Refit + focus whenever this tab becomes active (it was sized-but-hidden).
  createEffect(() => {
    if (props.active) queueMicrotask(() => { fitAndResize(); term?.focus(); });
  });

  onCleanup(() => {
    ro?.disconnect();
    props.unregister(props.tab.id);
    term?.dispose();
  });

  return <div class="term-host" classList={{ "term-host--active": props.active }} ref={host} />;
}
