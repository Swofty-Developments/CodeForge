/* Entry: bundle fonts locally (no CDN), register the global event listeners,
 * mount the app. */

import "@fontsource/dm-sans/400.css";
import "@fontsource/dm-sans/500.css";
import "@fontsource/dm-sans/600.css";
import "@fontsource/dm-sans/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "@fontsource/jetbrains-mono/600.css";

import { render } from "solid-js/web";
import App from "./App";
import {
  listenAgentEvent,
  listenIndexProgress,
  listenRepoChanged,
  listenTimelineEvent,
} from "./ipc";
import { appStore } from "./stores/app-store";

async function boot() {
  // One global listener per channel; reducers demux internally.
  await Promise.all([
    listenAgentEvent(appStore.handleAgentEvent),
    listenTimelineEvent(appStore.handleTimelineEvent),
    listenIndexProgress(appStore.handleIndexProgress),
    listenRepoChanged(appStore.handleRepoChanged),
  ]).catch(() => {
    // Not running inside Tauri (plain `vite dev` in a browser) — fine for UI work.
  });
}

void boot();

render(() => <App />, document.getElementById("app")!);
