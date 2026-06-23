#!/usr/bin/env bash
# Emit a marker-style receipt for a breed pipeline evaluation.
# Usage: pipeline-receipt.sh <breeds-csv> <fitness> <status> [ocel_path]
# Output: receipt JSON to stdout; validates with validate-receipt-chain.sh
set -uo pipefail

breeds_csv="${1:?usage: pipeline-receipt.sh <breeds-csv> <fitness> <status> [ocel_path]}"
fitness="${2:?fitness required}"
status="${3:?bounded-status required}"
ocel_path="${4:-none}"

# Validate status is bounded
case "$status" in
  ADMITTED|REFUSED|PARTIAL|UNKNOWN|BLOCKED|CANDIDATE|OPEN) ;;
  *)
    echo >&2 "REFUSED: invalid status '$status' — must be bounded"
    exit 1
    ;;
esac

# Build the raw_command string
raw_cmd="lsp-max-cli pipeline evaluate --breeds $breeds_csv"
[ "$ocel_path" != "none" ] && raw_cmd="$raw_cmd --ocel-path $ocel_path"

# Compute digest of (breeds + fitness + ocel_path)
content="${breeds_csv}|${fitness}|${ocel_path}"
if command -v sha256sum >/dev/null 2>&1; then
  digest="$(printf '%s' "$content" | sha256sum | awk '{print $1}')"
  alg="sha256"
elif command -v openssl >/dev/null 2>&1; then
  digest="$(printf '%s' "$content" | openssl dgst -sha256 | awk '{print $2}')"
  alg="sha256"
else
  echo >&2 "UNKNOWN: no sha256sum or openssl available"
  exit 0
fi

ts="$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo 'unknown')"

# Emit the marker-style receipt
jq -n \
  --arg bnd "-----BEGIN RECEIPT-----" \
  --arg chk "-----END RECEIPT-----" \
  --arg raw "$raw_cmd" \
  --arg dig "$digest" \
  --arg alg "$alg" \
  --arg st  "$status" \
  --arg fit "$fitness" \
  --arg brd "$breeds_csv" \
  --arg ts  "$ts" \
  --arg ocel "$ocel_path" \
  '{
    boundary: $bnd,
    checkpoint: $chk,
    raw_command: $raw,
    digest: $dig,
    digest_algorithm: $alg,
    status: $st,
    breeds: $brd,
    fitness: $fit,
    evaluated_at: $ts,
    ocel_path: $ocel
  }'
