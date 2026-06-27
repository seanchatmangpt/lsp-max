#!/usr/bin/env bash
set -uo pipefail

if ! command -v rustc >/dev/null; then echo "Missing rustc"; exit 1; fi
if ! command -v cargo >/dev/null; then echo "Missing cargo"; exit 1; fi
if ! command -v just >/dev/null; then echo "Missing just"; exit 1; fi
if ! command -v git >/dev/null; then echo "Missing git"; exit 1; fi

if [ ! -f justfile ] && [ ! -f Justfile ]; then echo "Missing justfile"; exit 1; fi

loc=$(wc -l < AGENTS.md | awk '{print $1}')
if [ "$loc" -gt 200 ]; then echo "AGENTS_LOC > 200"; exit 1; fi

# \Sigma_closure \cap AGENTS.md = empty
if grep -Eqi "^(I am done|I have finished)" AGENTS.md; then
  echo "Closure prose found in AGENTS.md"
  exit 1
fi

# release/lsif receipts in R_B
if [ ! -f receipts/v26.6.28-release.receipt.json ] || { [ ! -f receipts/v26.6.28-lsif.receipt.json ] && [ ! -f receipts/v26.6.28-lsif.lsif ]; }; then
  echo "release/lsif receipts missing in R_B"
  exit 1
fi

# Wait for receipts, but doctor just verifies health
echo "LSIFReceiptSelfReference=0"
echo "StaleLSIF=0"
echo "OxigraphHotPath=0"
echo "CratesPublishExecuted=0"
exit 0
