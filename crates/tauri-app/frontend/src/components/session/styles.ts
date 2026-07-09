/* Session-pane stylesheet injector. The CSS is split across three sibling chunks
 * (chrome / stream / composer) to keep each file small; they're concatenated and
 * injected once (id-guarded) since SessionPane, MessageStream, blocks.tsx and the
 * composer share these class names. Keyframes live in global.css. */

import { CHROME_CSS } from "./styles-chrome";
import { STREAM_CSS } from "./styles-stream";
import { COMPOSER_CSS } from "./styles-composer";

const STYLE_ID = "session-styles";

export function injectSessionStyles(): void {
  if (typeof document === "undefined" || document.getElementById(STYLE_ID)) return;
  const style = document.createElement("style");
  style.id = STYLE_ID;
  style.textContent = CHROME_CSS + STREAM_CSS + COMPOSER_CSS;
  document.head.appendChild(style);
}
