#!/bin/sh
# CodeForge hook forwarder: pipes Claude Code hook JSON (stdin) to the local
# daemon. Always exits 0 so hooks never block Claude.
#
# Three named states, never collapsed:
#   1. App closed (no daemon.json)      -> honest no-op, nothing to deliver.
#   2. Daemon up, POST succeeds         -> delivered, temp removed.
#   3. Daemon should be up but POST fails (portless manifest or unreachable)
#      -> spool the payload to runtime/spool/ for the daemon to drain on next
#         start, instead of dropping it on the floor.
ROOT="${CLAUDE_PROJECT_DIR:-.}"
RUNTIME="$ROOT/.codeforge/runtime/daemon.json"
SPOOL="$ROOT/.codeforge/runtime/spool"

# State 1: app closed. No daemon has ever advertised a port here; drop silently.
[ -f "$RUNTIME" ] || exit 0

# Capture stdin once — we may need to both POST it and spool it.
mkdir -p "$SPOOL" 2>/dev/null || exit 0
TMP=$(mktemp "$SPOOL/spool.XXXXXX") || exit 0
cat > "$TMP"

PORT=$(sed -n 's/.*"port"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$RUNTIME" | head -n 1)
if [ -n "$PORT" ] && curl -s --max-time 2 -X POST -H "Content-Type: application/json" \
    --data-binary @"$TMP" "http://127.0.0.1:$PORT/hooks/event" >/dev/null 2>&1; then
  # State 2: delivered.
  rm -f "$TMP"
else
  # State 3: daemon should be up but this POST failed — leave $TMP spooled.
  :
fi
exit 0
