import { render } from "solid-js/web";
import { onMount } from "solid-js";
import { App } from "./App";
import { appStore } from "./stores/app-store";
import { listenAgentEvent } from "./ipc";

function Root() {
  onMount(async () => {
    await appStore.loadData();
    await listenAgentEvent(appStore.handleAgentEvent);
    await appStore.requestNotificationPermission();
  });

  return <App />;
}

render(() => <Root />, document.getElementById("app")!);
