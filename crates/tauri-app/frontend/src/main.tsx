import { render } from "solid-js/web";
import { onMount } from "solid-js";
import { App } from "./App";
import { appStore } from "./stores/app-store";
import { listenAgentEvent } from "./ipc";
import { loadSavedTheme } from "./themes";

function Root() {
  onMount(async () => {
    // Load theme first (non-blocking)
    loadSavedTheme().catch(() => {});
    // Core initialization
    try {
      await appStore.loadData();
      await listenAgentEvent(appStore.handleAgentEvent);
    } catch (e) {
      console.error("Init failed:", e);
    }
    // Non-critical
    appStore.requestNotificationPermission().catch(() => {});
  });

  return <App />;
}

render(() => <Root />, document.getElementById("app")!);
