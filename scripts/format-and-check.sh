#!/bin/bash
#
# scripts/format-and-check.sh
#
# Runs cargo fmt and cargo clippy with strict warnings-as-errors flag.
# If either step fails, exits with code 1 and reports the failure point.
#
# This script ensures consistent code style and lint compliance before commits.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo -e "${MAGENTA}========================================${NC}"
echo -e "${MAGENTA}Format & Lint Check${NC}"
echo -e "${MAGENTA}========================================${NC}"

FAILED=0

# ============================================================================
# Step 1: cargo fmt --all
# ============================================================================
echo -e "\n${BLUE}[Step 1/2] Running cargo fmt --all...${NC}"

if cargo fmt --all; then
  echo -e "${GREEN}✓ Formatting passed.${NC}"
else
  echo -e "${RED}✗ Formatting failed.${NC}"
  FAILED=1
fi

# ============================================================================
# Step 2: cargo clippy with strict warnings
# ============================================================================
echo -e "\n${BLUE}[Step 2/2] Running cargo clippy --all-targets -D warnings...${NC}"

if cargo clippy --workspace --all-targets --all-features -- -D warnings; then
  echo -e "${GREEN}✓ Lint checks passed.${NC}"
else
  echo -e "${RED}✗ Lint checks failed.${NC}"
  FAILED=1
fi

# ============================================================================
# Summary
# ============================================================================
echo -e "\n${MAGENTA}========================================${NC}"
if [ $FAILED -eq 0 ]; then
  echo -e "${GREEN}✓ All format and lint checks passed!${NC}"
  echo -e "${MAGENTA}========================================${NC}"
  exit 0
else
  echo -e "${RED}✗ Format and lint check failed!${NC}"
  echo -e "${YELLOW}Please fix the issues above and try again.${NC}"
  echo -e "${MAGENTA}========================================${NC}"
  exit 1
fi
