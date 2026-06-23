#!/usr/bin/env bash
# Bootstrap the lsp-max workspace.
#
# This workspace does not build standalone: Cargo path deps and [patch.crates-io]
# require three sibling checkouts next to the repo root:
#   ../lsp-types-max  ../wasm4pm-compat  ../wasm4pm
# This script makes them PRESENT (cloning the ones that are MISSING) and reports
# environment readiness. It is idempotent and non-interactive — safe to run
# repeatedly and from a SessionStart hook.
#
# Usage:
#   scripts/bootstrap.sh           # clone missing siblings, then report
#   scripts/bootstrap.sh --check   # report only; never clone (read-only doctor)
#
# Sibling source mirrors CI: https://github.com/<org>/<repo>.git
#   LSP_MAX_SIBLING_ORG    override the org (default: seanchatmangpt)
#   LSP_MAX_SIBLING_BASE   override the full clone base (e.g. git@github.com:seanchatmangpt)
#   SIBLING_REPO_TOKEN     PAT with read access, for private sibling repos
#   LSP_MAX_BOOTSTRAP_DEPTH  shallow-clone depth (default: full clone, matching CI)

set -uo pipefail

ORG="${LSP_MAX_SIBLING_ORG:-seanchatmangpt}"
SIBLINGS=(lsp-types-max wasm4pm-compat wasm4pm)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PARENT_DIR="$(cd "$REPO_ROOT/.." && pwd)"

MODE="setup"
case "${1:-}" in
  --check | check | doctor) MODE="check" ;;
esac

if [ -t 1 ]; then
  C_OK=$'\033[0;32m'; C_WARN=$'\033[1;33m'; C_ERR=$'\033[0;31m'; C_DIM=$'\033[0;36m'; C_NC=$'\033[0m'
else
  C_OK=""; C_WARN=""; C_ERR=""; C_DIM=""; C_NC=""
fi

note()   { printf '%s\n' "$*"; }
status() { printf '  %-24s %s\n' "$1" "$2"; }

clone_base() {
  if [ -n "${LSP_MAX_SIBLING_BASE:-}" ]; then
    printf '%s' "$LSP_MAX_SIBLING_BASE"
  elif [ -n "${SIBLING_REPO_TOKEN:-}" ]; then
    printf 'https://x-access-token:%s@github.com/%s' "$SIBLING_REPO_TOKEN" "$ORG"
  else
    printf 'https://github.com/%s' "$ORG"
  fi
}

missing=0

note "${C_DIM}lsp-max bootstrap${C_NC}  (mode: $MODE)"
note "  repo root:        $REPO_ROOT"
note "  siblings under:   $PARENT_DIR"
note ""
note "Sibling repositories (build prerequisite):"

for repo in "${SIBLINGS[@]}"; do
  dest="$PARENT_DIR/$repo"
  if [ -d "$dest" ] && [ -n "$(ls -A "$dest" 2>/dev/null)" ]; then
    status "$repo" "${C_OK}PRESENT${C_NC}"
    continue
  fi
  if [ "$MODE" = "check" ]; then
    status "$repo" "${C_ERR}MISSING${C_NC} (run: just setup)"
    missing=$((missing + 1))
    continue
  fi
  safe_url="https://github.com/$ORG/$repo.git"
  note "  ${C_DIM}cloning $repo from $safe_url ...${C_NC}"
  clone_args=(--quiet)
  [ -n "${LSP_MAX_BOOTSTRAP_DEPTH:-}" ] && clone_args+=(--depth "$LSP_MAX_BOOTSTRAP_DEPTH")
  if git clone "${clone_args[@]}" "$(clone_base)/$repo.git" "$dest"; then
    status "$repo" "${C_OK}CLONED${C_NC}"
  else
    status "$repo" "${C_ERR}CLONE FAILED${C_NC}"
    missing=$((missing + 1))
  fi
done

note ""
note "Toolchain:"
toolchain_missing=0
check_cmd() {
  if command -v "$1" >/dev/null 2>&1; then
    status "$1" "${C_OK}PRESENT${C_NC}  $("$1" --version 2>/dev/null | head -1)"
  else
    status "$1" "${C_WARN}NOT FOUND${C_NC}  ($2)"
    return 1
  fi
}
check_cmd cargo "install Rust via https://rustup.rs" || toolchain_missing=$((toolchain_missing + 1))
check_cmd just  "optional entry points: cargo install just" || true

note ""
note "Optional (agent features):"
if [ -n "${LSP_MAX_API_KEY:-}${OPENAI_API_KEY:-}" ]; then
  status "LLM API key" "${C_OK}SET${C_NC}"
else
  status "LLM API key" "${C_DIM}unset${C_NC}  (set LSP_MAX_API_KEY for 'lsp-max-cli agent')"
fi

note ""
if [ "$missing" -eq 0 ] && [ "$toolchain_missing" -eq 0 ]; then
  note "${C_OK}Environment READY.${C_NC}  Next: just test   (or: cargo test --workspace)"
  exit 0
elif [ "$missing" -eq 0 ]; then
  note "${C_WARN}Siblings PRESENT; Rust toolchain NOT FOUND.${C_NC}  Install via https://rustup.rs"
  exit 1
elif [ "$MODE" = "check" ]; then
  note "${C_WARN}$missing sibling(s) MISSING.${C_NC}  Run: just setup"
  exit 1
else
  note "${C_ERR}$missing sibling(s) could not be cloned.${C_NC}  Check network / SIBLING_REPO_TOKEN, then re-run: just setup"
  exit 1
fi
