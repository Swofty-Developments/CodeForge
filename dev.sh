#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# Handle CTRL+C gracefully - forward to child process
cleanup() {
    if [ -n "${PID:-}" ]; then
        kill -SIGTERM "$PID" 2>/dev/null || true
        wait "$PID" 2>/dev/null || true
    fi
    exit 0
}
trap cleanup SIGINT SIGTERM

echo "Starting CodeForge..."
cargo run --manifest-path Cargo.toml -p codeforge-app -- "$@" &
PID=$!
wait "$PID"
