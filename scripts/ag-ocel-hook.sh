#!/usr/bin/env bash
set -euo pipefail

HOOK="${1:-unknown}"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

mkdir -p .antigravity/ocel .antigravity/payloads

TS="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
NS="$(date -u +"%Y%m%dT%H%M%S").$$"
EVENT_ID="${NS}.${HOOK}"
PAYLOAD=".antigravity/payloads/${EVENT_ID}.stdin"
EVENT=".antigravity/ocel/${EVENT_ID}.json"
JSONL=".antigravity/ocel/events.jsonl"

cat > "$PAYLOAD" || true

cargo run -p lsp-max-cli --quiet --bin ag-ocel-hook -- "$HOOK" "$TS" "$EVENT_ID" "$PAYLOAD" "$EVENT" "$JSONL"

if [ "$HOOK" = "Stop" ]; then
  echo "\[" >&2
  echo "OCEL=.antigravity/ocel/events.jsonl" >&2
  echo "\]" >&2

  echo "\[" >&2
  echo "GitDeltaStats=OCELTail" >&2
  echo "\]" >&2

  tail -5 "$JSONL" >&2 || true
fi

exit 0
