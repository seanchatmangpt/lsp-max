#!/bin/bash
# PreToolUse ANDON gate. Exit 0 = proceed. Exit 1 = blocked with structured reason.
# Passes through when lsp-max-cli is not yet built (fresh session).
command -v lsp-max-cli >/dev/null 2>&1 || exit 0

lsp-max-cli gate check --format=agent-context
