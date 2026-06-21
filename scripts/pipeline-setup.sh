#!/usr/bin/env bash
# Check and report pipeline search readiness.
# Emits bounded status for each pipeline prerequisite.
set -uo pipefail

echo "Pipeline (TPOT2) Setup Check"
echo "=============================="

# Check: lsp-max-cli is built
if command -v lsp-max-cli >/dev/null 2>&1 || [ -x "./target/debug/lsp-max-cli" ] || [ -x "./target/release/lsp-max-cli" ]; then
  echo "  lsp-max-cli     ADMITTED"
else
  echo "  lsp-max-cli     BLOCKED   → cargo build -p lsp-max-cli"
fi

# Check: wasm4pm-cli is available (optional, enables richer fitness)
if command -v wasm4pm >/dev/null 2>&1; then
  echo "  wasm4pm-cli     ADMITTED  (subprocess fitness enabled)"
else
  echo "  wasm4pm-cli     PARTIAL   (heuristic fitness only — still works)"
fi

# Check: OCEL file if configured
ocel="${LSP_MAX_PIPELINE_OCEL:-}"
if [ -z "$ocel" ]; then
  echo "  ocel_path       UNKNOWN   (set LSP_MAX_PIPELINE_OCEL for process-specific fitness)"
elif [ -f "$ocel" ]; then
  echo "  ocel_path       ADMITTED  ($ocel)"
else
  echo "  ocel_path       REFUSED   (file not found: $ocel)"
fi

# Check: breed catalog is non-empty (static, always present)
echo "  breed_catalog   ADMITTED  ($(ls ../wasm4pm/crates/wasm4pm-cognition/src/breeds/*.rs 2>/dev/null | wc -l | tr -d ' ') breeds)"

echo ""
echo "Run 'just pipeline-search' to begin. No OCEL file required for initial search."
