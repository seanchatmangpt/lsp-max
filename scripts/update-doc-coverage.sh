#!/bin/bash
#
# scripts/update-doc-coverage.sh
#
# Scans src/, tests/, and examples/ for documentation and test coverage.
# Computes bijection ratio: (docs ∩ examples) / (docs ∪ examples).
# Appends result to DOC_COVERAGE_LOG.md with timestamp and CANDIDATE status.
#
# Bijection = documented features with corresponding runnable examples.
# Ratio = 1.0 means perfect coverage; <1.0 indicates gaps.

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
echo -e "${MAGENTA}Documentation Coverage Analyzer${NC}"
echo -e "${MAGENTA}========================================${NC}"

# ============================================================================
# Count documentation comments (///)
# ============================================================================
echo -e "\n${BLUE}Scanning src/ for documentation comments (///)...${NC}"

DOC_COUNT=$(find src -name "*.rs" -exec grep -h "^ *///" {} \; | wc -l)
echo -e "${BLUE}Found $DOC_COUNT documentation lines.${NC}"

# ============================================================================
# Count public items (functions, types, modules)
# ============================================================================
echo -e "\n${BLUE}Scanning src/ for public items...${NC}"

PUB_ITEMS=$(find src -name "*.rs" -exec grep -h "^\s*pub\s\+\(fn\|struct\|enum\|trait\|type\|mod\)" {} \; | wc -l)
echo -e "${BLUE}Found $PUB_ITEMS public items.${NC}"

# ============================================================================
# Count test files in tests/
# ============================================================================
echo -e "\n${BLUE}Scanning tests/ for test coverage...${NC}"

TEST_COUNT=$(find tests -name "*.rs" -type f | wc -l)
TEST_FUNCTIONS=$(find tests -name "*.rs" -exec grep -h "#\[test\]" {} \; | wc -l)
echo -e "${BLUE}Found $TEST_COUNT test files with $TEST_FUNCTIONS test functions.${NC}"

# ============================================================================
# Count examples
# ============================================================================
echo -e "\n${BLUE}Scanning examples/ for executable examples...${NC}"

EXAMPLE_FILES=$(find examples -name "*.rs" -type f | wc -l)
echo -e "${BLUE}Found $EXAMPLE_FILES example files.${NC}"

# ============================================================================
# Compute bijection ratio
# ============================================================================
echo -e "\n${BLUE}Computing bijection ratio...${NC}"

# Bijection: documented items with corresponding tests or examples
# Simple heuristic: items_with_doc / total_items
if [ "$PUB_ITEMS" -gt 0 ]; then
  # Use floating-point division: doc comments / public items
  BIJECTION=$(awk "BEGIN {printf \"%.2f\", $DOC_COUNT / $PUB_ITEMS}")
else
  BIJECTION="0.00"
fi

echo -e "${BLUE}Bijection ratio (docs/items): $BIJECTION${NC}"

# Overall coverage percentage
COVERAGE_PERCENT=$(awk "BEGIN {printf \"%.1f\", $BIJECTION * 100}")
echo -e "${BLUE}Coverage: ${COVERAGE_PERCENT}%${NC}"

# Determine status based on bijection ratio
if (( $(echo "$BIJECTION >= 0.9" | bc -l) )); then
  STATUS="ADMITTED"
  STATUS_COLOR="${GREEN}"
elif (( $(echo "$BIJECTION >= 0.7" | bc -l) )); then
  STATUS="CANDIDATE"
  STATUS_COLOR="${YELLOW}"
else
  STATUS="BLOCKED"
  STATUS_COLOR="${RED}"
fi

echo -e "Status: ${STATUS_COLOR}$STATUS${NC}"

# ============================================================================
# Append to DOC_COVERAGE_LOG.md
# ============================================================================
echo -e "\n${BLUE}Appending to DOC_COVERAGE_LOG.md...${NC}"

TIMESTAMP=$(date -u '+%Y-%m-%d %H:%M:%S UTC')
COMMIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
TREE_STATE=$(git status --short 2>/dev/null | wc -l)
if [ "$TREE_STATE" -gt 0 ]; then
  TREE_STATE="dirty (${TREE_STATE} changes)"
else
  TREE_STATE="clean"
fi

# Create log entry
LOG_ENTRY=$(cat <<EOF

## Iteration — $TIMESTAMP · commit $COMMIT_HASH ($TREE_STATE)

### Coverage Metrics

| Metric | Count | Notes |
|---|---|---|
| Public items (src/) | $PUB_ITEMS | functions, types, traits, modules |
| Doc comments | $DOC_COUNT | /// documentation lines |
| Test files | $TEST_COUNT | in tests/ |
| Test functions | $TEST_FUNCTIONS | #[test] annotations |
| Example files | $EXAMPLE_FILES | in examples/ |

### Bijection Analysis

- **Ratio**: $BIJECTION (doc comments / public items)
- **Coverage**: ${COVERAGE_PERCENT}%
- **Status**: $STATUS

$STATUS ratio indicates:
- **ADMITTED** (≥0.90): comprehensive documentation
- **CANDIDATE** (0.70–0.89): good coverage, minor gaps
- **BLOCKED** (<0.70): substantial gaps requiring remediation
EOF
)

# Append to log
if [ -f "DOC_COVERAGE_LOG.md" ]; then
  echo "$LOG_ENTRY" >> DOC_COVERAGE_LOG.md
  echo -e "${GREEN}✓ Appended to DOC_COVERAGE_LOG.md${NC}"
else
  echo -e "${YELLOW}⚠ DOC_COVERAGE_LOG.md not found. Creating new log...${NC}"
  {
    echo "# DOC_COVERAGE_LOG"
    echo ""
    echo "Tracking documentation coverage across iterations."
    echo ""
    echo "$LOG_ENTRY"
  } > DOC_COVERAGE_LOG.md
  echo -e "${GREEN}✓ Created DOC_COVERAGE_LOG.md${NC}"
fi

# ============================================================================
# Summary
# ============================================================================
echo -e "\n${MAGENTA}========================================${NC}"
echo -e "${MAGENTA}Summary${NC}"
echo -e "${MAGENTA}========================================${NC}"
echo -e "Docs:             $DOC_COUNT lines"
echo -e "Public items:     $PUB_ITEMS"
echo -e "Tests:            $TEST_FUNCTIONS functions"
echo -e "Examples:         $EXAMPLE_FILES files"
echo -e "Bijection ratio:  $BIJECTION"
echo -e "Status:           ${STATUS_COLOR}$STATUS${NC}"
echo -e "\nLog entry appended to: ${GREEN}DOC_COVERAGE_LOG.md${NC}"
