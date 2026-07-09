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
import { appStore } from "./stores/app-store";

// One idempotent registration for all event channels; reducers demux internally.
void appStore.initListeners();

render(() => <App />, document.getElementById("app")!);
