#!/usr/bin/env bash
# install-gate-hook.sh -- BL10 idempotent pre-commit installer.
# Wires .git/hooks/pre-commit to run scripts/gate-branchless-lsp.py,
# preserving an existing hook by chaining it (legacy hook runs first).
# Usage: install-gate-hook.sh [--git-dir <path>]
set -euo pipefail

if [ "${1:-}" = "--git-dir" ]; then
  GIT_DIR="${2:?--git-dir requires a path}"
else
  GIT_DIR="$(git rev-parse --git-dir)"
fi
GIT_DIR="$(cd "$GIT_DIR" && pwd)"

# Locate the gate script next to this installer (absolute).
HERE="$(cd "$(dirname "$0")" && pwd)"
GATE="$HERE/gate-branchless-lsp.py"
[ -f "$GATE" ] || { echo "install-gate-hook: gate script missing: $GATE" >&2; exit 2; }

HOOK="$GIT_DIR/hooks/pre-commit"
mkdir -p "$GIT_DIR/hooks"

if [ -f "$HOOK" ] && grep -q "branchless-lsp-admission-gate" "$HOOK"; then
  echo "install-gate-hook: already installed at $HOOK (idempotent no-op)"
  exit 0
fi

if [ -f "$HOOK" ]; then
  echo "install-gate-hook: chaining existing pre-commit hook"
  cat >> "$HOOK" <<'EOF'

# --- branchless-lsp-admission-gate (chained by install-gate-hook.sh) ---
EOF
  chmod +x "$HOOK"
else
  printf '#!/bin/sh\n' > "$HOOK"
  printf '\n# --- branchless-lsp-admission-gate (installed by install-gate-hook.sh) ---\n' >> "$HOOK"
fi
printf '"%s" "$@"\n' "$GATE" >> "$HOOK"
chmod +x "$HOOK"
echo "install-gate-hook: installed $HOOK -> $GATE"
