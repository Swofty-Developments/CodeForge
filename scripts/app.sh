#!/usr/bin/env bash
# Build the standalone release .app and open it — the demo/daily-driver path.
# Slower than scripts/dev.sh (full release compile) but fully self-contained:
# no vite server, no dev ports.
set -euo pipefail
cd "$(dirname "$0")/../crates/tauri-app"

frontend/node_modules/.bin/tauri build --bundles app

# The bundle lands under the workspace target dir; check both layouts.
for app in target/release/bundle/macos/CodeForge.app \
           ../../target/release/bundle/macos/CodeForge.app; do
  if [ -d "$app" ]; then
    open "$app"
    exit 0
  fi
done
echo "app.sh: built, but CodeForge.app not found under target/release/bundle" >&2
exit 1
