#!/usr/bin/env bash
# Fast local (re)launch: vite dev server + `tauri dev` (debug build, HMR).
#
# Defaults to :5183 so it never fights nexode-web on :5173. Override with
# CODEFORGE_DEV_PORT. Re-running the script replaces the previous instance.
set -euo pipefail
cd "$(dirname "$0")/.."

PORT="${CODEFORGE_DEV_PORT:-5183}"
VITE_LOG="${TMPDIR:-/tmp}/codeforge-vite.log"

"$(dirname "$0")/stop.sh" >/dev/null 2>&1 || true

echo "codeforge dev: vite on :$PORT (log: $VITE_LOG)"
(cd crates/tauri-app/frontend && exec npx vite --port "$PORT" --strictPort) \
  >"$VITE_LOG" 2>&1 &
VITE_PID=$!

for _ in $(seq 1 60); do
  if curl -sf "http://localhost:$PORT" >/dev/null 2>&1; then break; fi
  if ! kill -0 "$VITE_PID" 2>/dev/null; then
    echo "codeforge dev: vite exited early — $VITE_LOG:" >&2
    tail -20 "$VITE_LOG" >&2
    exit 1
  fi
  sleep 0.5
done

echo "codeforge dev: launching tauri (debug)"
cd crates/tauri-app
exec frontend/node_modules/.bin/tauri dev \
  --config "{\"build\":{\"devUrl\":\"http://localhost:$PORT\"}}"
