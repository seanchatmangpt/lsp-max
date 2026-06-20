#!/bin/bash
#
# scripts/health-check.sh
#
# Runs all developer experience checks in sequence:
#  1. Law compliance (check-law-compliance.sh)
#  2. Format and lint (format-and-check.sh)
#  3. Benchmark analysis (bench-hot-paths.sh)
#  4. Documentation coverage (update-doc-coverage.sh)
#
# Prints colored summary (green/yellow/red) and exits with overall health code.
#
# Exit codes:
#   0 = All checks passed (ADMITTED)
#   1 = At least one check failed (BLOCKED)
#   2 = Some checks CANDIDATE (partial)

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

# Track overall status
OVERALL_STATUS="ADMITTED"
EXIT_CODE=0

# ============================================================================
# HEALTH CHECK RUNNER
# ============================================================================
echo -e "${MAGENTA}╔════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}║      Codebase Health Check            ║${NC}"
echo -e "${MAGENTA}╚════════════════════════════════════════╝${NC}"

# Colors for status badges
declare -A STATUS_COLORS=(
  ["ADMITTED"]="${GREEN}"
  ["CANDIDATE"]="${YELLOW}"
  ["BLOCKED"]="${RED}"
)

declare -A CHECK_RESULTS

# ============================================================================
# Check 1: Law Compliance
# ============================================================================
echo -e "\n${BLUE}[1/4] Law Compliance Check${NC}"
echo -e "${BLUE}────────────────────────────────────────${NC}"

if bash "$SCRIPTS_DIR/check-law-compliance.sh" 2>&1; then
  CHECK_RESULTS["law_compliance"]="ADMITTED"
  echo -e "${GREEN}✓ ADMITTED${NC}"
else
  CHECK_RESULTS["law_compliance"]="BLOCKED"
  OVERALL_STATUS="BLOCKED"
  EXIT_CODE=1
  echo -e "${RED}✗ BLOCKED${NC}"
fi

# ============================================================================
# Check 2: Format and Lint
# ============================================================================
echo -e "\n${BLUE}[2/4] Format & Lint Check${NC}"
echo -e "${BLUE}────────────────────────────────────────${NC}"

if bash "$SCRIPTS_DIR/format-and-check.sh" 2>&1; then
  CHECK_RESULTS["format_lint"]="ADMITTED"
  echo -e "${GREEN}✓ ADMITTED${NC}"
else
  CHECK_RESULTS["format_lint"]="BLOCKED"
  OVERALL_STATUS="BLOCKED"
  EXIT_CODE=1
  echo -e "${RED}✗ BLOCKED${NC}"
fi

# ============================================================================
# Check 3: Benchmark Analysis
# ============================================================================
echo -e "\n${BLUE}[3/4] Benchmark Analysis${NC}"
echo -e "${BLUE}────────────────────────────────────────${NC}"

if bash "$SCRIPTS_DIR/bench-hot-paths.sh" 2>&1; then
  CHECK_RESULTS["benchmarks"]="ADMITTED"
  echo -e "${GREEN}✓ ADMITTED${NC}"
else
  # Benchmarks failing is a regression concern but not a blocker for build
  CHECK_RESULTS["benchmarks"]="CANDIDATE"
  if [ "$OVERALL_STATUS" = "ADMITTED" ]; then
    OVERALL_STATUS="CANDIDATE"
    EXIT_CODE=2
  fi
  echo -e "${YELLOW}⚠ CANDIDATE (regressions detected)${NC}"
fi

# ============================================================================
# Check 4: Documentation Coverage
# ============================================================================
echo -e "\n${BLUE}[4/4] Documentation Coverage${NC}"
echo -e "${BLUE}────────────────────────────────────────${NC}"

if bash "$SCRIPTS_DIR/update-doc-coverage.sh" 2>&1; then
  # Doc coverage is informational; always succeeds
  CHECK_RESULTS["doc_coverage"]="ADMITTED"
  echo -e "${GREEN}✓ Log updated${NC}"
else
  CHECK_RESULTS["doc_coverage"]="CANDIDATE"
  echo -e "${YELLOW}⚠ Log update encountered issues${NC}"
fi

# ============================================================================
# Summary Dashboard
# ============================================================================
echo -e "\n${MAGENTA}╔════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}║      Health Check Summary              ║${NC}"
echo -e "${MAGENTA}╚════════════════════════════════════════╝${NC}"

echo ""
echo -e "Law Compliance:        ${STATUS_COLORS[${CHECK_RESULTS[law_compliance]:-UNKNOWN}]}${CHECK_RESULTS[law_compliance]:-UNKNOWN}${NC}"
echo -e "Format & Lint:         ${STATUS_COLORS[${CHECK_RESULTS[format_lint]:-UNKNOWN}]}${CHECK_RESULTS[format_lint]:-UNKNOWN}${NC}"
echo -e "Benchmark Analysis:    ${STATUS_COLORS[${CHECK_RESULTS[benchmarks]:-UNKNOWN}]}${CHECK_RESULTS[benchmarks]:-UNKNOWN}${NC}"
echo -e "Doc Coverage:          ${STATUS_COLORS[${CHECK_RESULTS[doc_coverage]:-UNKNOWN}]}${CHECK_RESULTS[doc_coverage]:-UNKNOWN}${NC}"

echo ""
echo -e "Overall Status:        ${STATUS_COLORS[$OVERALL_STATUS]}$OVERALL_STATUS${NC}"
echo ""

if [ "$OVERALL_STATUS" = "ADMITTED" ]; then
  echo -e "${GREEN}═══════════════════════════════════════${NC}"
  echo -e "${GREEN}✓ Codebase is healthy and ready!${NC}"
  echo -e "${GREEN}═══════════════════════════════════════${NC}"
elif [ "$OVERALL_STATUS" = "CANDIDATE" ]; then
  echo -e "${YELLOW}═══════════════════════════════════════${NC}"
  echo -e "${YELLOW}⚠ Codebase has some issues to review${NC}"
  echo -e "${YELLOW}═══════════════════════════════════════${NC}"
else
  echo -e "${RED}═══════════════════════════════════════${NC}"
  echo -e "${RED}✗ Codebase has blocking issues!${NC}"
  echo -e "${RED}═══════════════════════════════════════${NC}"
fi

echo ""
exit $EXIT_CODE
