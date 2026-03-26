#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# ─── Configuration ──────────────────────────────────────────────
MAX_MEM_MB=2048        # Kill if exceeds 2GB RAM
MAX_CPU_PERCENT=80     # CPU limit (only with cgroup)
TIMEOUT_MINUTES=60     # Auto-kill after 1 hour

# ─── Force software rendering (prevents GPU lockups) ────────────
export WGPU_BACKEND=gl
export ICED_BACKEND=tiny-skia
export MESA_GL_VERSION_OVERRIDE=3.3
# Disable GPU compositing
export LIBGL_ALWAYS_SOFTWARE=1

# ─── Build first (so build errors don't happen inside sandbox) ──
echo "Building CodeForge..."
cargo build -p codeforge-app 2>&1
BINARY="./target/debug/codeforge-app"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    exit 1
fi

# ─── Cleanup handler ───────────────────────────────────────────
cleanup() {
    echo ""
    echo "Shutting down CodeForge..."
    if [ -n "${PID:-}" ]; then
        kill -SIGTERM "$PID" 2>/dev/null || true
        # Wait up to 5 seconds for graceful shutdown
        for i in $(seq 1 50); do
            kill -0 "$PID" 2>/dev/null || break
            sleep 0.1
        done
        # Force kill if still alive
        kill -SIGKILL "$PID" 2>/dev/null || true
    fi
    exit 0
}
trap cleanup SIGINT SIGTERM EXIT

# ─── Launch with resource limits ───────────────────────────────
echo "Starting CodeForge (software rendering, ${MAX_MEM_MB}MB memory limit)..."

if command -v systemd-run &>/dev/null; then
    # Best option: systemd-run provides proper cgroup isolation
    systemd-run --user --scope \
        --property=MemoryMax="${MAX_MEM_MB}M" \
        --property=MemorySwapMax=0 \
        --property=CPUQuota="${MAX_CPU_PERCENT}%" \
        --property=TimeoutStopSec=5 \
        --description="CodeForge IDE" \
        "$BINARY" "$@" &
    PID=$!
else
    # Fallback: ulimit-based limits (less robust but works everywhere)
    (
        # Soft memory limit via ulimit (in KB)
        ulimit -v $(( MAX_MEM_MB * 1024 )) 2>/dev/null || true
        # CPU time limit (seconds)
        ulimit -t $(( TIMEOUT_MINUTES * 60 )) 2>/dev/null || true
        exec "$BINARY" "$@"
    ) &
    PID=$!
fi

echo "PID: $PID"

# ─── Background watchdog (kills if memory exceeds limit) ───────
(
    while kill -0 "$PID" 2>/dev/null; do
        sleep 5
        if [ -f "/proc/$PID/status" ]; then
            RSS_KB=$(grep "VmRSS:" "/proc/$PID/status" 2>/dev/null | awk '{print $2}' || echo 0)
            RSS_MB=$(( RSS_KB / 1024 ))
            if [ "$RSS_MB" -gt "$MAX_MEM_MB" ] 2>/dev/null; then
                echo "WATCHDOG: CodeForge exceeded ${MAX_MEM_MB}MB (using ${RSS_MB}MB). Killing."
                kill -SIGKILL "$PID" 2>/dev/null || true
                exit 1
            fi
        fi
    done
) &
WATCHDOG_PID=$!

# Wait for app to exit
wait "$PID" 2>/dev/null
EXIT_CODE=$?
kill "$WATCHDOG_PID" 2>/dev/null || true

if [ "$EXIT_CODE" -eq 137 ]; then
    echo "CodeForge was killed (OOM or watchdog)."
elif [ "$EXIT_CODE" -ne 0 ]; then
    echo "CodeForge exited with code $EXIT_CODE"
fi

exit "$EXIT_CODE"
