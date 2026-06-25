#!/usr/bin/env bash
# Spawn lsp-max-compositor in the background if not already running.
# Called from SessionStart hook after discover-lsp-chains.sh.
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"
BINARY="$ROOT/target/debug/lsp-max-compositor"
LOGFILE="$ROOT/.claude/compositor.log"
PIDFILE="$ROOT/.claude/compositor.pid"

# Kill stale compositor if pid file points to dead process
if [[ -f "$PIDFILE" ]]; then
  OLD_PID="$(cat "$PIDFILE" 2>/dev/null || echo '')"
  if [[ -n "$OLD_PID" ]] && ! kill -0 "$OLD_PID" 2>/dev/null; then
    rm -f "$PIDFILE"
  fi
fi

# Already running — emit status and exit
if [[ -f "$PIDFILE" ]]; then
  PID="$(cat "$PIDFILE")"
  echo "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"lsp-max-compositor ADMITTED (PID=$PID)\"}}"
  exit 0
fi

# Binary missing — try building it (best-effort, don't block session start)
if [[ ! -x "$BINARY" ]]; then
  if command -v cargo &>/dev/null; then
    cargo build -p lsp-max-compositor --manifest-path "$ROOT/Cargo.toml" \
      >> "$LOGFILE" 2>&1 &
    BUILD_PID=$!
    echo "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"lsp-max-compositor OPEN — build in progress (PID=$BUILD_PID); run: cargo build -p lsp-max-compositor\"}}"
  else
    echo "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"lsp-max-compositor OPEN — binary missing, cargo not found\"}}"
  fi
  exit 0
fi

# Launch compositor over stdio (Claude Code connects via .lsp.json; we also log stderr)
"$BINARY" 2>>"$LOGFILE" &
COMPOSITOR_PID=$!
echo "$COMPOSITOR_PID" > "$PIDFILE"

# Export PID for subsequent hooks if CLAUDE_ENV_FILE is set
if [[ -n "${CLAUDE_ENV_FILE:-}" ]]; then
  echo "LSP_MAX_COMPOSITOR_PID=$COMPOSITOR_PID" >> "$CLAUDE_ENV_FILE"
fi

echo "{\"hookSpecificOutput\":{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"lsp-max-compositor CANDIDATE (PID=$COMPOSITOR_PID) → $LOGFILE\"}}"
