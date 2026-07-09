//! Static assets the integration kit writes into a repo: the hook forwarder
//! script, the CLAUDE.md guidance block, and the marker/matcher constants.

/// Markers delimiting the FeatureForge-owned section of CLAUDE.md.
pub(crate) const CLAUDE_MD_START: &str = "<!-- featureforge:start -->";
pub(crate) const CLAUDE_MD_END: &str = "<!-- featureforge:end -->";

/// Hook command installed into `.claude/settings.json`. `$CLAUDE_PROJECT_DIR`
/// is expanded by Claude Code at hook time.
pub(crate) const HOOK_COMMAND: &str = "\"$CLAUDE_PROJECT_DIR\"/.featureforge/hooks/forward.sh";
/// Substring identifying our hook entries across re-installs.
pub(crate) const HOOK_MARKER: &str = ".featureforge/hooks/forward.sh";
pub(crate) const POST_TOOL_USE_MATCHER: &str = "Edit|Write|MultiEdit|NotebookEdit|Bash";

pub(crate) const FORWARD_SH: &str = r#"#!/bin/sh
# FeatureForge hook forwarder: pipes Claude Code hook JSON (stdin) to the local
# daemon. Always exits 0 so hooks never block Claude.
#
# Three named states, never collapsed:
#   1. App closed (no daemon.json)      -> honest no-op, nothing to deliver.
#   2. Daemon up, POST succeeds         -> delivered, temp removed.
#   3. Daemon should be up but POST fails (portless manifest or unreachable)
#      -> spool the payload to runtime/spool/ for the daemon to drain on next
#         start, instead of dropping it on the floor.
ROOT="${CLAUDE_PROJECT_DIR:-.}"
RUNTIME="$ROOT/.featureforge/runtime/daemon.json"
SPOOL="$ROOT/.featureforge/runtime/spool"

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
"#;

pub(crate) const CLAUDE_MD_BODY: &str = r#"## FeatureForge

This repository is indexed by FeatureForge. **Before exploring the codebase**, consult the
`featureforge` MCP tools — they are faster and more accurate than searching from scratch:

- `list_features` — every feature in this repo with a short description
- `get_feature(slug)` — one feature's description, entry points, and key files
- `which_features(path)` — which features a file path belongs to
- `feature_timeline(slug)` — recent changes to a feature, newest first

Record significant decisions and findings as you work via `record_note(text, feature_slugs)`
so they land on the repo timeline for future sessions."#;
