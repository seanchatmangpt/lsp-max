#!/usr/bin/env bash
# Validate a receipt artifact's structure against the project's receipt law
# (CLAUDE.md "Reception Validation Failures"): boundary marker, checkpoint
# closure, a valid digest, the bound fields, and a bounded status word.
#
# Emits ONLY bounded statuses (ADMITTED / REFUSED / PARTIAL / UNKNOWN) — never
# "done" / "VERIFIED" / victory language. This checks STRUCTURE; digest
# freshness (re-running raw_command) is reported UNKNOWN, not asserted.
#
# Exit: 0 for ADMITTED/UNKNOWN, 1 for REFUSED/PARTIAL.
set -uo pipefail

R="${1:?usage: validate-receipt-chain.sh <receipt.json>}"
[ -f "$R" ] || { echo "UNKNOWN: no such file: $R"; exit 0; }
jq -e . "$R" >/dev/null 2>&1 || { echo "UNKNOWN: not valid JSON: $R"; exit 0; }

get() { jq -r --arg k "$1" '.[$k] // empty' "$R"; }
boundary="$(get boundary)"

# Shape gate: only marker-style admission receipts declare a `boundary`. Other
# shapes (e.g. demoted human-report summaries that disclaim admission authority)
# are not assessed by this validator — report UNKNOWN, never REFUSED, so a shape
# we do not understand is not collapsed into a polarity.
if [ -z "$boundary" ]; then
  echo "UNKNOWN: no boundary field — not a marker-style admission receipt; not assessed ($R)"
  exit 0
fi

checkpoint="$(get checkpoint)"
digest="$(get digest)"
alg="$(get digest_algorithm)"
st="$(get status)"

# Boundary marker + checkpoint closure (CLAUDE.md: invalid without them).
if [ "$boundary" != "-----BEGIN RECEIPT-----" ] || [ "$checkpoint" != "-----END RECEIPT-----" ]; then
  echo "REFUSED: boundary/checkpoint markers missing or malformed ($R)"
  exit 1
fi

# Digest must be a 64-hex value (SHA-256 / BLAKE3-256).
if ! printf '%s' "$digest" | grep -qE '^[0-9a-f]{64}$'; then
  echo "REFUSED: digest is not a 64-hex value ($R)"
  exit 1
fi

# Required bound fields.
miss=()
for f in digest_algorithm digest raw_command; do
  [ -z "$(get "$f")" ] && miss+=("$f")
done
if [ "${#miss[@]}" -gt 0 ]; then
  echo "PARTIAL: missing bound field(s): ${miss[*]} ($R)"
  exit 1
fi

# A status word, if present, must be bounded — never victory language.
if [ -n "$st" ] && ! printf '%s' "$st" | grep -qE '^(ADMITTED|REFUSED|UNKNOWN|PARTIAL|BLOCKED|CANDIDATE|OPEN)$'; then
  echo "REFUSED: status '$st' is not a bounded status ($R)"
  exit 1
fi

echo "ADMITTED: structure + boundary + ${alg:-digest} format bound${st:+ (status=$st)} ($R); digest-freshness UNKNOWN (source not re-run)"
exit 0
