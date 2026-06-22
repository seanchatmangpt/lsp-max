#!/usr/bin/env bash
# SessionStart hook — fires on startup/resume/clear/compact.
# Emits hookSpecificOutput JSON; Claude Code injects additionalContext before first prompt.
# Must complete in <10s. No victory language.
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo /home/user/lsp-max)"
BRANCH="$(git -C "$ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo UNKNOWN)"
LAST_COMMIT="$(git -C "$ROOT" log -1 --format='%h %s' 2>/dev/null || echo UNKNOWN)"
DIRTY="$(git -C "$ROOT" status --short 2>/dev/null | wc -l | tr -d ' ')"
PARENT="$(dirname "$ROOT")"

# Check tools
JUST_STATUS="$(just --version &>/dev/null && echo ADMITTED || echo OPEN)"
CLI_STATUS="$(lsp-max-cli --version &>/dev/null && echo ADMITTED || echo OPEN)"

# Check sibling repos
LSP_TYPES_STATUS="$([ -d "$PARENT/lsp-types-max/.git" ] && echo ADMITTED || echo OPEN)"
WASM4PM_COMPAT_STATUS="$([ -d "$PARENT/wasm4pm-compat/.git" ] && echo ADMITTED || echo OPEN)"
WASM4PM_STATUS="$([ -d "$PARENT/wasm4pm/.git" ] && echo ADMITTED || echo OPEN)"

CONTEXT="lsp-max session — branch: $BRANCH | last commit: $LAST_COMMIT | dirty files: $DIRTY
Toolchain: just=$JUST_STATUS  lsp-max-cli=$CLI_STATUS
Sibling repos: lsp-types-max=$LSP_TYPES_STATUS  wasm4pm-compat=$WASM4PM_COMPAT_STATUS  wasm4pm=$WASM4PM_STATUS"

if [[ "$LSP_TYPES_STATUS" == "OPEN" || "$WASM4PM_COMPAT_STATUS" == "OPEN" || \
      "$WASM4PM_STATUS" == "OPEN" || "$JUST_STATUS" == "OPEN" || "$CLI_STATUS" == "OPEN" ]]; then
  CONTEXT="$CONTEXT
ACTION REQUIRED: run \`bash .claude/setup.sh\` to bootstrap missing tools/repos before building.
Workspace build requires all three sibling repos at $PARENT/."
fi

# Persist LSPMAX_ROOT for all subsequent Bash commands in this session
if [[ -n "${CLAUDE_ENV_FILE:-}" ]]; then
  echo "LSPMAX_ROOT=$ROOT" >> "$CLAUDE_ENV_FILE"
fi

# Emit hookSpecificOutput JSON — additionalContext injected before first prompt
if command -v python3 &>/dev/null; then
  CONTEXT_JSON="$(printf '%s' "$CONTEXT" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')"
  cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": $CONTEXT_JSON,
    "sessionTitle": "lsp-max — $BRANCH"
  }
}
EOF
else
  # Fallback: plain stdout also becomes additionalContext
  echo "$CONTEXT"
fi
