#!/usr/bin/env bash
# One-time environment bootstrap for lsp-max web/cloud sessions.
# Safe to re-run. Emits bounded statuses (ADMITTED/CANDIDATE/OPEN), no victory language.
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo /home/user/lsp-max)"
PARENT="$(dirname "$ROOT")"

echo "=== lsp-max environment bootstrap ==="
echo "ROOT: $ROOT"
echo "PARENT: $PARENT"
echo ""

# ── 1. Install just ────────────────────────────────────────────────────────────
echo "── just ──"
if just --version &>/dev/null; then
  echo "  just: ADMITTED ($(just --version 2>&1 | head -1))"
else
  echo "  just: OPEN — installing via cargo..."
  if cargo install just 2>&1 | tail -3; then
    echo "  just: CANDIDATE (installed; verify with: just --version)"
  else
    echo "  just: OPEN (install attempt returned non-zero; check cargo output above)"
  fi
fi
echo ""

# ── 2. Clone sibling repos ────────────────────────────────────────────────────
echo "── sibling repos ──"

clone_if_absent() {
  local name="$1" url="$2" dest="$PARENT/$name"
  if [ -d "$dest/.git" ]; then
    echo "  $name: ADMITTED ($dest)"
  else
    echo "  $name: OPEN — cloning from $url..."
    if git clone "$url" "$dest" 2>&1 | tail -3; then
      echo "  $name: CANDIDATE (cloned; build not verified)"
    else
      echo "  $name: OPEN (clone returned non-zero; check output above)"
    fi
  fi
}

clone_if_absent "lsp-types-max"   "https://github.com/seanchatmangpt/lsp-types-max"
clone_if_absent "wasm4pm-compat"  "https://github.com/seanchatmangpt/wasm4pm-compat"
clone_if_absent "wasm4pm"         "https://github.com/seanchatmangpt/wasm4pm"
echo ""

# ── 3. Build lsp-max-cli ──────────────────────────────────────────────────────
echo "── lsp-max-cli ──"
if lsp-max-cli --version &>/dev/null; then
  echo "  lsp-max-cli: ADMITTED ($(lsp-max-cli --version 2>&1 | head -1))"
else
  echo "  lsp-max-cli: OPEN — building via cargo install..."
  if cargo install --path "$ROOT/crates/lsp-max-cli" 2>&1 | tail -5; then
    echo "  lsp-max-cli: CANDIDATE (installed; verify with: lsp-max-cli --version)"
  else
    echo "  lsp-max-cli: OPEN (build returned non-zero; sibling repos may be required first)"
  fi
fi
echo ""

echo "=== bootstrap status summary ==="
echo "  just:             $(just --version &>/dev/null && echo ADMITTED || echo OPEN)"
echo "  lsp-types-max:    $([ -d "$PARENT/lsp-types-max/.git" ] && echo ADMITTED || echo OPEN)"
echo "  wasm4pm-compat:   $([ -d "$PARENT/wasm4pm-compat/.git" ] && echo ADMITTED || echo OPEN)"
echo "  wasm4pm:          $([ -d "$PARENT/wasm4pm/.git" ] && echo ADMITTED || echo OPEN)"
echo "  lsp-max-cli:      $(lsp-max-cli --version &>/dev/null && echo ADMITTED || echo OPEN)"
echo ""
echo "When all sibling repos are ADMITTED, run: cargo build --workspace"
echo "When lsp-max-cli is ADMITTED, the ANDON gate (PreToolUse) will function."
