#!/bin/bash
#
# scripts/check-law-compliance.sh
#
# Scans the codebase for law violations as defined in CLAUDE.md and AGENTS.md:
# 1. Plain `tower_lsp` or `tower_lsp_` references (forbidden, use `lsp-max`)
# 2. TOWER_LSP_MAX_* environment variables (forbidden naming)
# 3. Victory language ("done", "solved", "all clean", "fully admitted", "guaranteed")
#
# Reports violations with file:line for easy remediation.
# Exit code: 0 if clean, 1 if violations found.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

VIOLATIONS=0
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Law Compliance Scanner${NC}"
echo -e "${BLUE}========================================${NC}"

# ============================================================================
# Check 1: Plain tower_lsp references (forbidden except in negative controls)
# ============================================================================
echo -e "\n${BLUE}[1] Scanning for plain 'tower_lsp' references...${NC}"

PLAIN_TOWER=$(rg -n "tower_lsp\b|tower-lsp\b|tower_lsp_\w+" \
  --glob '!target/**' --glob '!.git/**' --glob '!.cargo/**' \
  --glob '!vendor/**' --glob '!**/node_modules/**' \
  --glob '!Cargo.lock' --glob '!package-lock.json' \
  --glob '!**/*.wasm' --glob '!**/*.log' \
  . || true)

if [ -n "$PLAIN_TOWER" ]; then
  # Filter out known negative-control fixtures and documentation
  FILTERED=$(echo "$PLAIN_TOWER" | grep -v \
    -e "examples/anti-llm-cheat-lsp" \
    -e "test.*tower_lsp\|test_tower\|tests/" \
    -e "\.md:" \
    -e "AGENTS\.md" \
    -e "CLAUDE\.md" \
    -e "comment.*tower_lsp\|tower-lsp" \
    || true)

  if [ -n "$FILTERED" ]; then
    echo -e "${RED}✗ VIOLATION: Plain 'tower_lsp' found (use 'lsp-max' instead)${NC}"
    echo "$FILTERED" | head -20
    if [ "$(echo "$FILTERED" | wc -l)" -gt 20 ]; then
      echo "... ($(echo "$FILTERED" | wc -l) total violations)"
    fi
    ((VIOLATIONS++))
  else
    echo -e "${GREEN}✓ No plain 'tower_lsp' references in code.${NC}"
  fi
else
  echo -e "${GREEN}✓ No plain 'tower_lsp' references found.${NC}"
fi

# ============================================================================
# Check 2: TOWER_LSP_MAX_* environment variables (forbidden)
# ============================================================================
echo -e "\n${BLUE}[2] Scanning for TOWER_LSP_MAX_* environment variables...${NC}"

TOWER_LSP_MAX=$(rg -n "TOWER_LSP_MAX_\w+" \
  --glob '!target/**' --glob '!.git/**' --glob '!.cargo/**' \
  --glob '!vendor/**' --glob '!**/node_modules/**' \
  --glob '!Cargo.lock' --glob '!package-lock.json' \
  --glob '!**/*.wasm' --glob '!**/*.log' \
  . || true)

if [ -n "$TOWER_LSP_MAX" ]; then
  echo -e "${RED}✗ VIOLATION: TOWER_LSP_MAX_* environment variable naming found${NC}"
  echo "$TOWER_LSP_MAX" | head -20
  if [ "$(echo "$TOWER_LSP_MAX" | wc -l)" -gt 20 ]; then
    echo "... ($(echo "$TOWER_LSP_MAX" | wc -l) total violations)"
  fi
  ((VIOLATIONS++))
else
  echo -e "${GREEN}✓ No TOWER_LSP_MAX_* variables found.${NC}"
fi

# ============================================================================
# Check 3: Victory language (done, solved, all clean, fully admitted, guaranteed)
# ============================================================================
echo -e "\n${BLUE}[3] Scanning for victory language...${NC}"

# Victory language patterns: done, solved, all clean, fully admitted, guaranteed
# Exclude common legitimate uses like "done();" in Rust, documentation context
VICTORY=$(rg -n -i "\b(done|solved|all clean|fully admitted|guaranteed)\b" \
  --glob '!target/**' --glob '!.git/**' --glob '!.cargo/**' \
  --glob '!vendor/**' --glob '!**/node_modules/**' \
  --glob '!Cargo.lock' --glob '!package-lock.json' \
  --glob '!**/*.wasm' --glob '!**/*.log' \
  . || true)

if [ -n "$VICTORY" ]; then
  # Filter out false positives: function calls, legitimate contexts
  FILTERED=$(echo "$VICTORY" | grep -v \
    -e "fn done()\|\.done()" \
    -e "test.*solved" \
    -e "//.*done\|//.*solved" \
    -e "all_clean\|all-clean" \
    -e "test_.*_solved" \
    -e "comment.*victory\|victory language" \
    -e "\.md:" \
    || true)

  if [ -n "$FILTERED" ]; then
    echo -e "${RED}✗ VIOLATION: Victory language detected (use bounded statuses only)${NC}"
    echo -e "${YELLOW}Valid statuses: ADMITTED, CANDIDATE, BLOCKED, REFUSED, UNKNOWN, PARTIAL, OPEN${NC}"
    echo "$FILTERED" | head -20
    if [ "$(echo "$FILTERED" | wc -l)" -gt 20 ]; then
      echo "... ($(echo "$FILTERED" | wc -l) total violations)"
    fi
    ((VIOLATIONS++))
  else
    echo -e "${GREEN}✓ No victory language in code.${NC}"
  fi
else
  echo -e "${GREEN}✓ No victory language found.${NC}"
fi

# ============================================================================
# Summary
# ============================================================================
echo -e "\n${BLUE}========================================${NC}"
if [ $VIOLATIONS -eq 0 ]; then
  echo -e "${GREEN}✓ All law compliance checks passed!${NC}"
  echo -e "${BLUE}========================================${NC}"
  exit 0
else
  echo -e "${RED}✗ Found $VIOLATIONS compliance violation(s)${NC}"
  echo -e "${BLUE}========================================${NC}"
  exit 1
fi
