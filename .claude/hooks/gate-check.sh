#!/bin/bash
# Resilient PreToolUse wrapper for lsp-max-cli gate check.
# Passes through (exit 0) when the binary is not yet built rather than
# blocking all tools in a fresh session where the binary doesn't exist yet.
command -v lsp-max-cli >/dev/null 2>&1 || exit 0
lsp-max-cli gate check
