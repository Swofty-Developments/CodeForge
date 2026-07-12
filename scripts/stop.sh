#!/usr/bin/env bash
# Stop a running CodeForge dev instance (tauri debug binary + its vite server).
set -uo pipefail

PORT="${CODEFORGE_DEV_PORT:-5183}"

pkill -f 'target/debug/codeforge-tauri' 2>/dev/null && echo "stopped codeforge-tauri (debug)"
pkill -f "vite --port $PORT" 2>/dev/null && echo "stopped vite on :$PORT"
exit 0
