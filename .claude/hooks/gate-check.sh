#!/bin/bash
# PreToolUse ANDON gate. Exit 0 = proceed. Exit 1 = blocked with structured reason.
# Passes through when lsp-max-cli is not yet built (fresh session).
command -v lsp-max-cli > /dev/null 2>&1 || exit 0

# Capture CLI output; it may emit agent-context JSON when blocked
CLI_OUT=$(lsp-max-cli gate check --format=agent-context 2>/dev/null)
CLI_EXIT=$?

if [ $CLI_EXIT -ne 0 ]; then
  # Read fitness snapshot for enriched context if available
  FITNESS_FILE="${FITNESS_PATH:-${CLAUDE_PROJECT_DIR:-.}/.claude/lsp-max-fitness.json}"
  REASON="ANDON gate blocked"
  FIRST_VIOLATION="{}"
  LAW_STATUS="BLOCKED"

  if [ -f "$FITNESS_FILE" ]; then
    LAW_STATUS=$(jq -r '.law_status // "BLOCKED"' "$FITNESS_FILE" 2>/dev/null || echo "BLOCKED")
    REASON=$(jq -r '"ANDON: " + (.violations[0].detail // "gate blocked")' "$FITNESS_FILE" 2>/dev/null || echo "ANDON gate blocked")
    FIRST_VIOLATION=$(jq -c '.violations[0] // {}' "$FITNESS_FILE" 2>/dev/null || echo "{}")
  fi

  # If CLI itself emitted non-empty JSON, prefer it; otherwise emit our own
  if [ -n "$CLI_OUT" ] && echo "$CLI_OUT" | jq . > /dev/null 2>&1; then
    echo "$CLI_OUT"
  else
    jq -n \
      --arg reason "$REASON" \
      --arg law_status "$LAW_STATUS" \
      --argjson first_violation "$FIRST_VIOLATION" \
      '{"decision":"block","reason":$reason,"routing_action":"halt_until_andon_clears","law_status":$law_status,"first_violation":$first_violation}'
  fi
  exit 1
fi

exit 0
