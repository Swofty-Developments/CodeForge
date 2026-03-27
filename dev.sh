#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

IMAGE_NAME="codeforge"
CONTAINER_NAME="codeforge-dev"
FORCE_BUILD=false
SHELL_MODE=false
NETWORK="none"

# ─── Parse flags ──────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --build)   FORCE_BUILD=true; shift ;;
        --shell)   SHELL_MODE=true; shift ;;
        --network) NETWORK="$2"; shift 2 ;;
        *)         break ;;
    esac
done

# ─── Build image if needed ────────────────────────────────────────
if $FORCE_BUILD || ! docker image inspect "$IMAGE_NAME" &>/dev/null; then
    echo "Building Docker image..."
    docker build -t "$IMAGE_NAME" .
fi

# ─── Construct run arguments ─────────────────────────────────────
RUN_ARGS=(
    --rm
    --name "$CONTAINER_NAME"
    --memory=2g
    --memory-swap=2g
    --cpus=2
    --pids-limit=100
    --network="$NETWORK"
    --security-opt=no-new-privileges
)

# X11 forwarding
if [ -n "${DISPLAY:-}" ]; then
    RUN_ARGS+=(
        -e DISPLAY="$DISPLAY"
        -v /tmp/.X11-unix:/tmp/.X11-unix:ro
    )
fi

# Wayland support
if [ -n "${WAYLAND_DISPLAY:-}" ] && [ -n "${XDG_RUNTIME_DIR:-}" ]; then
    RUN_ARGS+=(
        -e WAYLAND_DISPLAY="$WAYLAND_DISPLAY"
        -e XDG_RUNTIME_DIR=/tmp/xdg
        -v "${XDG_RUNTIME_DIR}/${WAYLAND_DISPLAY}:/tmp/xdg/${WAYLAND_DISPLAY}:ro"
    )
fi

# Persistence
CODEFORGE_DIR="${HOME}/.codeforge"
mkdir -p "$CODEFORGE_DIR"
RUN_ARGS+=(-v "$CODEFORGE_DIR:/home/codeforge/.codeforge")

# ─── Run ──────────────────────────────────────────────────────────
if $SHELL_MODE; then
    echo "Dropping into container shell..."
    exec docker run -it --entrypoint /bin/bash "${RUN_ARGS[@]}" "$IMAGE_NAME"
else
    echo "Starting CodeForge (Docker sandbox, 2GB mem, 2 CPUs)..."
    exec docker run -it "${RUN_ARGS[@]}" "$IMAGE_NAME" "$@"
fi
