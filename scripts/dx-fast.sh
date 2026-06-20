#!/usr/bin/env bash
#
# scripts/dx-fast.sh
#
# Sub-minute pre-push smoke. NOT a CI substitute (use scripts/preflight.sh as
# the "will CI pass?" oracle). This catches the cheap, high-frequency failures
# before a push:
#
#   1. fmt    cargo fmt -- --check         (whole workspace; fmt is near-instant)
#   2. clippy scripts/dx-changed.sh        (clippy only on changed crates)
#   3. laws   scripts/check-law-compliance.sh  (forbidden framework refs, overclaim language)
#
# fmt runs on the pinned toolchain from rust-toolchain.toml for CI fidelity;
# the clippy stage inherits the same pinning via dx-changed.sh.
#
# Usage:  scripts/dx-fast.sh [BASE_REF]   (BASE_REF forwarded to dx-changed.sh)
#
# Exit codes:
#   0 = all three stages ADMITTED
#   1 = at least one stage BLOCKED
#   2 = a stage returned UNKNOWN (precondition unmet) and none BLOCKED

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPTS_DIR="$PROJECT_ROOT/scripts"
cd "$PROJECT_ROOT"

BASE_REF="${1:-origin/master}"

# Resolve the pinned toolchain (shared with preflight / dx-changed).
TOOLCHAIN_FILE="$PROJECT_ROOT/rust-toolchain.toml"
PINNED_CHANNEL=""
if [ -f "$TOOLCHAIN_FILE" ]; then
  PINNED_CHANNEL="$(
    grep -E '^[[:space:]]*channel[[:space:]]*=' "$TOOLCHAIN_FILE" \
      | head -n 1 \
      | sed -E 's/.*=[[:space:]]*"([^"]+)".*/\1/'
  )"
fi
CARGO=(cargo)
if [ -n "$PINNED_CHANNEL" ] && rustup toolchain list 2>/dev/null | grep -q "$PINNED_CHANNEL"; then
  CARGO=(cargo "+${PINNED_CHANNEL}")
fi

echo -e "${MAGENTA}============================================================${NC}"
echo -e "${BLUE}dx-fast: pre-push smoke (fmt + changed-crate clippy + laws)${NC}"
echo -e "${MAGENTA}============================================================${NC}"

FMT_STATUS="UNKNOWN"
CLIPPY_STATUS="UNKNOWN"
LAW_STATUS="UNKNOWN"

# ----------------------------------------------------------------------------
# Stage 1: workspace fmt check (read-only).
# ----------------------------------------------------------------------------
echo -e "\n${BLUE}[1/3] cargo fmt -- --check${NC}"
if "${CARGO[@]}" fmt -- --check; then
  FMT_STATUS="ADMITTED"
else
  FMT_STATUS="BLOCKED"
fi

# ----------------------------------------------------------------------------
# Stage 2: clippy on changed crates only.
# dx-changed.sh exit: 0=ADMITTED, 1=BLOCKED, 2=UNKNOWN precondition.
# ----------------------------------------------------------------------------
echo -e "\n${BLUE}[2/3] clippy on changed crates (scripts/dx-changed.sh ${BASE_REF})${NC}"
set +e
bash "$SCRIPTS_DIR/dx-changed.sh" "$BASE_REF"
DX_RC=$?
set -e
case "$DX_RC" in
  0) CLIPPY_STATUS="ADMITTED" ;;
  1) CLIPPY_STATUS="BLOCKED" ;;
  *) CLIPPY_STATUS="UNKNOWN" ;;
esac

# ----------------------------------------------------------------------------
# Stage 3: law compliance scan.
# ----------------------------------------------------------------------------
echo -e "\n${BLUE}[3/3] scripts/check-law-compliance.sh${NC}"
set +e
bash "$SCRIPTS_DIR/check-law-compliance.sh"
LAW_RC=$?
set -e
if [ "$LAW_RC" -eq 0 ]; then
  LAW_STATUS="ADMITTED"
else
  LAW_STATUS="BLOCKED"
fi

# ----------------------------------------------------------------------------
# Bounded summary.
# ----------------------------------------------------------------------------
echo -e "\n${MAGENTA}============================================================${NC}"
echo -e "${BLUE}dx-fast summary${NC}"
echo -e "${MAGENTA}============================================================${NC}"
printf "%-22s %s\n" "STAGE" "STATUS"
for pair in "fmt (workspace):$FMT_STATUS" "clippy (changed):$CLIPPY_STATUS" "law-compliance:$LAW_STATUS"; do
  name="${pair%%:*}"; st="${pair##*:}"
  case "$st" in
    ADMITTED) col="${GREEN}" ;;
    BLOCKED)  col="${RED}" ;;
    *)        col="${YELLOW}" ;;
  esac
  printf "%-22s ${col}%s${NC}\n" "$name" "$st"
done
echo ""

if [ "$FMT_STATUS" = "BLOCKED" ] || [ "$CLIPPY_STATUS" = "BLOCKED" ] || [ "$LAW_STATUS" = "BLOCKED" ]; then
  echo -e "${RED}dx-fast verdict: BLOCKED${NC}"
  exit 1
fi
if [ "$CLIPPY_STATUS" = "UNKNOWN" ] || [ "$LAW_STATUS" = "UNKNOWN" ] || [ "$FMT_STATUS" = "UNKNOWN" ]; then
  echo -e "${YELLOW}dx-fast verdict: PARTIAL (a stage returned UNKNOWN; preconditions unmet)${NC}"
  exit 2
fi
echo -e "${GREEN}dx-fast verdict: ADMITTED (pre-push smoke passed; run scripts/preflight.sh for the full CI oracle)${NC}"
exit 0
