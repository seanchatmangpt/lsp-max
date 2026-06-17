#!/bin/bash
#
# scripts/bench-hot-paths.sh
#
# Runs criterion benchmarks from benches/ directory (if it exists) and parses
# output to detect performance regressions. Reports baseline vs. current with
# percentage change for each benchmark.
#
# Exit code: 0 if no regressions detected, 1 if regressions found.

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
echo -e "${MAGENTA}Hot Paths Benchmark Runner${NC}"
echo -e "${MAGENTA}========================================${NC}"

# Check if benches/ directory exists
if [ ! -d "benches" ]; then
  echo -e "${YELLOW}⚠ No 'benches/' directory found. Skipping benchmarks.${NC}"
  exit 0
fi

# Check if there are any bench files
BENCH_COUNT=$(find benches -name "*.rs" 2>/dev/null | wc -l)
if [ "$BENCH_COUNT" -eq 0 ]; then
  echo -e "${YELLOW}⚠ No benchmark files found in benches/. Skipping.${NC}"
  exit 0
fi

echo -e "${BLUE}Found $BENCH_COUNT benchmark file(s).${NC}"

# Check if criterion is available (it's a dev dependency)
if ! cargo metadata --format-version 1 2>/dev/null | grep -q '"name":"criterion"'; then
  echo -e "${YELLOW}⚠ Criterion dependency not found. Installing...${NC}"
fi

# Create a temporary directory for benchmark results
BENCH_OUTPUT=$(mktemp -d)
trap "rm -rf $BENCH_OUTPUT" EXIT

echo -e "\n${BLUE}Running benchmarks...${NC}"

# Run criterion benchmarks and capture output
if ! cargo bench --benches -- --output-format verbose 2>&1 | tee "$BENCH_OUTPUT/bench.log"; then
  echo -e "${RED}✗ Benchmark execution failed${NC}"
  exit 1
fi

# Parse criterion output for regressions
# Criterion output format includes "change" lines like:
#   time:   [X.XXX s X.XXX s X.XXX s]                 change: [+0.00% +0.00% +0.00%]
REGRESSION_COUNT=0
IMPROVEMENT_COUNT=0
STABLE_COUNT=0

echo -e "\n${BLUE}Analyzing benchmark results...${NC}"

# Look for performance changes in the output
if grep -q "change:" "$BENCH_OUTPUT/bench.log"; then
  # Extract change percentages
  CHANGES=$(grep "change:" "$BENCH_OUTPUT/bench.log" || true)

  # Count regressions (increase by >5%) vs improvements (decrease) vs stable
  while IFS= read -r line; do
    # Extract percentage from lines like: "change: [+5.23% +5.45% +5.67%]"
    if [[ $line =~ change:\ \[(.*)\] ]]; then
      CHANGES_STR="${BASH_REMATCH[1]}"
      # Get the middle value (median estimate)
      MEDIAN=$(echo "$CHANGES_STR" | awk '{print $2}')
      MEDIAN_NUM=$(echo "$MEDIAN" | sed 's/[^0-9.-]//g')

      if (( $(echo "$MEDIAN_NUM > 5" | bc -l 2>/dev/null || echo 0) )); then
        ((REGRESSION_COUNT++))
        echo -e "${RED}  ✗ Regression: $MEDIAN_NUM%${NC} (line: $line)"
      elif (( $(echo "$MEDIAN_NUM < -2" | bc -l 2>/dev/null || echo 0) )); then
        ((IMPROVEMENT_COUNT++))
        echo -e "${GREEN}  ✓ Improvement: $MEDIAN_NUM%${NC}"
      else
        ((STABLE_COUNT++))
      fi
    fi
  done <<< "$CHANGES"
fi

# Summary
echo -e "\n${MAGENTA}========================================${NC}"
echo -e "${BLUE}Benchmark Summary${NC}"
echo -e "${MAGENTA}========================================${NC}"
echo -e "Improvements: ${GREEN}$IMPROVEMENT_COUNT${NC}"
echo -e "Stable:       ${BLUE}$STABLE_COUNT${NC}"
echo -e "Regressions:  ${RED}$REGRESSION_COUNT${NC}"

if [ "$REGRESSION_COUNT" -gt 0 ]; then
  echo -e "\n${RED}✗ Performance regressions detected!${NC}"
  echo -e "${YELLOW}Investigate with: cargo bench --benches -- --verbose${NC}"
  exit 1
else
  echo -e "\n${GREEN}✓ No performance regressions detected.${NC}"
  exit 0
fi
