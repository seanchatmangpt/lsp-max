#!/usr/bin/env bash
# Integration test for pipeline-receipt.sh receipt emission.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Test 1: ADMITTED receipt has correct structure
receipt="$(bash scripts/pipeline-receipt.sh "cbr,ltl_monitor,asp" "0.85" "ADMITTED")"
echo "$receipt" | jq -e '.boundary == "-----BEGIN RECEIPT-----"' >/dev/null || { echo "REFUSED: boundary missing"; exit 1; }
echo "$receipt" | jq -e '.checkpoint == "-----END RECEIPT-----"' >/dev/null || { echo "REFUSED: checkpoint missing"; exit 1; }
echo "$receipt" | jq -e '.status == "ADMITTED"' >/dev/null || { echo "REFUSED: status wrong"; exit 1; }
echo "$receipt" | jq -e '.fitness' >/dev/null || { echo "REFUSED: fitness missing"; exit 1; }

# Test 2: validator recognizes receipt as ADMITTED
tmp="$(mktemp /tmp/pipeline-receipt-test-XXXXXX.json)"
echo "$receipt" > "$tmp"
result="$(bash scripts/validate-receipt-chain.sh "$tmp")"
rm -f "$tmp"
case "$result" in
  ADMITTED*) echo "ADMITTED: pipeline receipt validated" ;;
  *) echo "REFUSED: validate-receipt-chain reported: $result"; exit 1 ;;
esac

# Test 3: Invalid status is REFUSED
set +e
bad_out="$(bash scripts/pipeline-receipt.sh "cbr" "0.5" "COMPLETED" 2>&1)"
rc=$?
set -e
[ $rc -ne 0 ] || { echo "REFUSED: invalid status should fail"; exit 1; }

echo "ADMITTED: pipeline-receipt.sh test suite passed"
