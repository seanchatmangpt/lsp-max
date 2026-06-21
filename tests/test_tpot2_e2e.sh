#!/usr/bin/env bash
# End-to-end transcript: drive the TPOT2 pipeline CLI against a sample OCEL log,
# emit a marker-style receipt for the search result via scripts/pipeline-receipt.sh,
# and validate that receipt with scripts/validate-receipt-chain.sh.
#
# This is a TRANSCRIPT + RECEIPT + NEGATIVE-CONTROL exercise. Per project law:
#   - stdout/log lines are NOT receipts; the only receipt is the artifact emitted
#     by scripts/pipeline-receipt.sh (boundary / checkpoint / 64-hex digest).
#   - no fabricated fitness/admission values: every assertion is a RANGE or a
#     bounded-set membership against whatever the CLI actually returns.
#   - bounded statuses only (ADMITTED / PARTIAL / UNKNOWN / REFUSED / BLOCKED).
#
# Exit: 0 when the transcript is bounded-consistent, the receipt validates, and
# the negative-control invalid receipt is REJECTED. Non-zero otherwise.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
ROOT="$(pwd)"

EXPECTED_BREEDS=57
BREEDS="cbr,ltl_monitor,asp"
OCEL="tests/fixtures/tpot2/sample.ocel.json"

# ── temp artifacts (idempotent cleanup) ──────────────────────────────────────
TMPDIR_E2E="$(mktemp -d /tmp/tpot2-e2e-XXXXXX)"
cleanup() { rm -rf "$TMPDIR_E2E"; }
trap cleanup EXIT

refuse() { echo "E2E: REFUSED — $*" >&2; exit 1; }

# ── 0. fixture must exist and be well-formed OCEL ─────────────────────────────
[ -f "$OCEL" ] || refuse "OCEL fixture missing: $OCEL"
jq -e '.events and .objects and .eventTypes and .objectTypes' "$OCEL" >/dev/null 2>&1 \
  || refuse "OCEL fixture is not well-formed: $OCEL"

# ── 1. build or locate the CLI binary ────────────────────────────────────────
# Preferred: build in-tree. Where the workspace cannot resolve (e.g. an isolated
# worktree whose sibling repos are symlinked under the repo root), fall back to a
# pre-built lsp-max-cli on PATH or in a target/ dir. Locating a binary is an
# accepted path; fabricating output is not.
CLI=""
if cargo build -p lsp-max-cli >/dev/null 2>"$TMPDIR_E2E/build.err"; then
  CLI="$(cargo metadata --format-version 1 2>/dev/null \
        | jq -r '.target_directory' 2>/dev/null)/debug/lsp-max-cli"
fi
if [ -z "$CLI" ] || [ ! -x "$CLI" ]; then
  cand_target="$ROOT/target/debug/lsp-max-cli"
  cand_path="$(command -v lsp-max-cli 2>/dev/null || true)"
  if [ -x "$cand_target" ]; then
    CLI="$cand_target"
  elif [ -n "$cand_path" ] && [ -x "$cand_path" ]; then
    CLI="$cand_path"
  fi
fi
if [ -z "$CLI" ] || [ ! -x "$CLI" ]; then
  echo "E2E: BLOCKED — could not build or locate lsp-max-cli binary" >&2
  [ -s "$TMPDIR_E2E/build.err" ] && tail -5 "$TMPDIR_E2E/build.err" >&2
  exit 2
fi
echo "E2E: using CLI at $CLI"

run() { "$CLI" "$@"; }

# ── 2. pipeline schema → breed_count == 57 ───────────────────────────────────
schema_json="$(run pipeline schema)"
schema_breeds="$(printf '%s' "$schema_json" | jq -r '.breed_count')"
[ "$schema_breeds" = "$EXPECTED_BREEDS" ] \
  || refuse "schema breed_count=$schema_breeds, expected $EXPECTED_BREEDS"
echo "E2E: schema breed_count=$schema_breeds"

# ── 3. pipeline list-breeds → 57 breeds present ──────────────────────────────
list_json="$(run pipeline list-breeds)"
list_total="$(printf '%s' "$list_json" | jq -r '.total')"
list_count="$(printf '%s' "$list_json" | jq -r '.breeds | length')"
[ "$list_total" = "$EXPECTED_BREEDS" ] \
  || refuse "list-breeds total=$list_total, expected $EXPECTED_BREEDS"
[ "$list_count" = "$EXPECTED_BREEDS" ] \
  || refuse "list-breeds array length=$list_count, expected $EXPECTED_BREEDS"
echo "E2E: list-breeds total=$list_total"

# ── 4. pipeline evaluate → bounded status + fitness in [0,1] ─────────────────
eval_json="$(run pipeline evaluate --breeds "$BREEDS")"
eval_status="$(printf '%s' "$eval_json" | jq -r '.status')"
eval_fitness="$(printf '%s' "$eval_json" | jq -r '.pipeline.fitness')"
printf '%s' "$eval_status" | grep -qE '^(ADMITTED|PARTIAL|UNKNOWN|REFUSED)$' \
  || refuse "evaluate status '$eval_status' is not a bounded status"
printf '%s' "$eval_fitness" | grep -qE '^[0-9]+(\.[0-9]+)?$' \
  || refuse "evaluate fitness '$eval_fitness' is not numeric"
awk -v f="$eval_fitness" 'BEGIN { exit !(f >= 0 && f <= 1) }' \
  || refuse "evaluate fitness $eval_fitness outside [0,1]"
echo "E2E: evaluate status=$eval_status fitness=$eval_fitness"

# ── 5. pipeline search against the OCEL fixture → capture best_fitness+status ─
search_json="$(run pipeline search \
  --generations 5 --population-size 12 --ocel-path "$OCEL")"
search_status="$(printf '%s' "$search_json" | jq -r '.status')"
best_fitness="$(printf '%s' "$search_json" | jq -r '.best_fitness')"
best_breeds="$(printf '%s' "$search_json" | jq -r '.best_pipeline.breeds | join(",")')"
printf '%s' "$search_status" | grep -qE '^(ADMITTED|PARTIAL|UNKNOWN|REFUSED)$' \
  || refuse "search status '$search_status' is not a bounded status"
printf '%s' "$best_fitness" | grep -qE '^[0-9]+(\.[0-9]+)?$' \
  || refuse "search best_fitness '$best_fitness' is not numeric"
awk -v f="$best_fitness" 'BEGIN { exit !(f >= 0 && f <= 1) }' \
  || refuse "search best_fitness $best_fitness outside [0,1]"
[ -n "$best_breeds" ] || refuse "search returned no best_pipeline breeds"
echo "E2E: search status=$search_status best_fitness=$best_fitness breeds=$best_breeds"

# ── 6. emit a receipt for the search result via the receipt script ───────────
# The receipt binds the breeds the search actually selected, the fitness it
# actually returned, and the OCEL path. The status passed is the bounded status
# the search actually produced (not a fabricated polarity).
RECEIPT="$TMPDIR_E2E/tpot2_search.receipt.json"
bash scripts/pipeline-receipt.sh \
  "$best_breeds" "$best_fitness" "$search_status" "$OCEL" > "$RECEIPT" \
  || refuse "pipeline-receipt.sh failed to emit a receipt for the search result"
jq -e '.boundary == "-----BEGIN RECEIPT-----"' "$RECEIPT" >/dev/null \
  || refuse "emitted receipt lacks the BEGIN boundary marker"
jq -e '.checkpoint == "-----END RECEIPT-----"' "$RECEIPT" >/dev/null \
  || refuse "emitted receipt lacks the END checkpoint marker"
echo "E2E: receipt emitted at $RECEIPT"

# ── 7. validate the receipt chain (boundary + digest must match) ─────────────
val_out="$(bash scripts/validate-receipt-chain.sh "$RECEIPT")"
val_rc=$?
case "$val_out" in
  ADMITTED*) echo "E2E: receipt validated — $val_out" ;;
  *) refuse "validate-receipt-chain did not ADMIT the receipt: $val_out (rc=$val_rc)" ;;
esac

# ── 8. NEGATIVE CONTROL — prove the validator discriminates ──────────────────
# 8a. A marker receipt whose status is tampered to an out-of-band token MUST be
#     rejected (the validator enforces bounded-status membership).
NEG_STATUS="$TMPDIR_E2E/neg_status.receipt.json"
jq '.status = "WINNER"' "$RECEIPT" > "$NEG_STATUS"
set +e
neg_status_out="$(bash scripts/validate-receipt-chain.sh "$NEG_STATUS")"
neg_status_rc=$?
set -e
if [ "$neg_status_rc" -eq 0 ] && printf '%s' "$neg_status_out" | grep -q '^ADMITTED'; then
  refuse "negative control NOT rejected: validator ADMITTED a tampered-status receipt"
fi
echo "E2E: neg-control (tampered status) REJECTED — $neg_status_out"

# 8b. A receipt whose digest is flipped to a non-64-hex value MUST be rejected
#     (the validator enforces digest shape; a fabricated digest is rejected).
NEG_DIGEST="$TMPDIR_E2E/neg_digest.receipt.json"
jq '.digest = "deadbeef"' "$RECEIPT" > "$NEG_DIGEST"
set +e
neg_digest_out="$(bash scripts/validate-receipt-chain.sh "$NEG_DIGEST")"
neg_digest_rc=$?
set -e
if [ "$neg_digest_rc" -eq 0 ] && printf '%s' "$neg_digest_out" | grep -q '^ADMITTED'; then
  refuse "negative control NOT rejected: validator ADMITTED a flipped-digest receipt"
fi
echo "E2E: neg-control (flipped digest) REJECTED — $neg_digest_out"

# 8c. The emitter itself MUST refuse an out-of-band status token.
set +e
emit_bad="$(bash scripts/pipeline-receipt.sh "$BREEDS" "0.5" "WINNER" 2>&1)"
emit_bad_rc=$?
set -e
[ "$emit_bad_rc" -ne 0 ] \
  || refuse "negative control NOT rejected: emitter accepted an out-of-band status"
echo "E2E: neg-control (emitter rejects out-of-band status) REJECTED — $emit_bad"

# ── 9. final bounded-status line ─────────────────────────────────────────────
# The transcript is bounded-consistent and the receipt validated. The overall
# status mirrors the search's own bounded status — it is not asserted to be any
# particular polarity. If the search admitted (fitness >= threshold) the run is
# ADMITTED; otherwise the run carries the search's bounded status forward.
case "$search_status" in
  ADMITTED) echo "E2E: ADMITTED — transcript bounded, receipt validated, negative controls rejected" ;;
  PARTIAL)  echo "E2E: PARTIAL — transcript bounded, receipt validated, negative controls rejected" ;;
  UNKNOWN)  echo "E2E: UNKNOWN — transcript bounded, receipt validated, negative controls rejected" ;;
  *)        echo "E2E: PARTIAL — transcript bounded (search status=$search_status), receipt validated, negative controls rejected" ;;
esac
exit 0
