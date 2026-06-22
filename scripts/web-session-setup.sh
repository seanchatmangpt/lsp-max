#!/usr/bin/env bash
# SessionStart hook for Claude Code web / remote sessions.
#
# The full lsp-max workspace cannot build without sibling checkouts
# (../lsp-types-max, ../wasm4pm-compat, ../wasm4pm), which are absent in web
# sessions (a single repo is cloned). This hook detects that condition and
# injects guidance so an agent does not waste a turn on `cargo build --workspace`
# and instead verifies the self-contained subsystem it can actually run.
#
# Wire it into .claude/settings.json (the agent does not self-modify that file):
#
#   "SessionStart": [
#     { "matcher": "startup|resume",
#       "hooks": [ { "type": "command",
#                    "command": "\"$CLAUDE_PROJECT_DIR\"/scripts/web-session-setup.sh" } ] }
#   ]
#
# Per the hooks reference, SessionStart stdout (plain text, or a
# hookSpecificOutput.additionalContext JSON block) is injected into context
# before the first prompt. This script emits the JSON form when jq is available.
set -uo pipefail

ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

siblings_present="yes"
for s in ../lsp-types-max ../wasm4pm-compat ../wasm4pm; do
  [ -d "$ROOT/$s" ] || siblings_present="no"
done

if [ "$siblings_present" = "yes" ]; then
  ctx="lsp-max: sibling repos present. The full workspace can build; use 'just test' / 'just dx-polish'."
else
  ctx="lsp-max workspace health: sibling repos (../lsp-types-max, ../wasm4pm-compat, ../wasm4pm) are ABSENT, so 'cargo build --workspace' and 'just test' are BLOCKED in this session. \
The self-contained optimizer subsystem (src/pipeline: catalog, types, search, fitness, pareto, ocel, phase) depends only on serde/serde_json/std and IS verifiable here: run 'bash scripts/tpot2-harness-verify.sh' to compile + test + clippy it in a throwaway crate and emit a validated receipt. \
Protocol/runtime crates that import lsp-types-max remain BLOCKED until siblings are checked out; treat their build status as BLOCKED, not REFUSED."
fi

if command -v jq >/dev/null 2>&1; then
  jq -n --arg c "$ctx" \
    '{hookSpecificOutput: {hookEventName: "SessionStart", additionalContext: $c}}'
else
  printf '%s\n' "$ctx"
fi
exit 0
