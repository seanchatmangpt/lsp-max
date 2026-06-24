#!/usr/bin/env bash
# Gate check preamble for subagent prompts.
#
# Every subagent spawned via the Agent tool MUST run this as its first Bash
# action. PreToolUse hooks enforcing Λ_CD^runtime do not cross Agent session
# boundaries — this is a structural gap (RFC-1, status: OPEN).
#
# Usage: bash scripts/subagent-gate-check.sh
# Exit 0 — gate clear; the subagent may proceed.
# Exit 1 — ANDON gate ACTIVE; subagent blocked until all WASM4PM-*/ANTI-LLM-*/GGEN-* errors clear.
set -e

# If lsp-max-cli is not in PATH (fresh env, binary not yet built), pass through.
# Absence of the binary means the compositor is not running; gate is not enforced.
if ! command -v lsp-max-cli >/dev/null 2>&1; then
    echo "lsp-max-cli not found — compositor absent, gate not enforced. Proceeding." >&2
    exit 0
fi

lsp-max-cli gate check || {
    echo "ANDON gate ACTIVE — subagent blocked." >&2
    echo "Resolve all WASM4PM-*/ANTI-LLM-*/GGEN-* errors before proceeding." >&2
    echo "Run: lsp-max-cli gate list  (for active code families and agent_scope)" >&2
    exit 1
}

echo "Gate clear. Proceeding."
