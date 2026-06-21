#!/usr/bin/env bash
# Determinism & reproducibility WITNESS for the TPOT2 pipeline CLI, exercised
# THROUGH THE BINARY (lsp-max-cli) — not in unit tests.
#
# Why this exists, at the binary boundary:
#   A receipt only means something if the run it binds is reproducible. The
#   search PRNG is a fixed-seed xorshift64 and the heuristic fitness is a pure
#   categorical function, so identical args MUST produce byte-identical output.
#   This script witnesses that property where an agent/CI actually invokes it
#   (stdout of the compiled binary), then witnesses that the receipt artifact's
#   SHA256 digest is STABLE across identical inputs and DISTINCT for different
#   inputs — i.e. the digest binds its inputs and is not a constant.
#
# Per project law:
#   - stdout/log lines are NOT receipts; the only receipt is the artifact emitted
#     by scripts/pipeline-receipt.sh (boundary / checkpoint / 64-hex digest).
#   - no fabricated fitness/admission values: this asserts EQUALITY between
#     identical runs and DIFFERENCE for the negative control — never a specific
#     fitness number.
#   - bounded statuses only (ADMITTED / PARTIAL / UNKNOWN / REFUSED / BLOCKED).
#
# Exit: 0 when identical args reproduce byte-identical search+evaluate output,
# the receipt digest is stable across identical inputs, the receipt validates,
# and the different-input negative-control receipt yields a DISTINCT digest.
# Non-zero otherwise.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
ROOT="$(pwd)"

# Fixed search parameters — identical across the two determinism runs.
GENERATIONS=8
POPULATION=16
BREEDS="cbr,ltl_monitor,asp"
# Negative-control breed set: a DIFFERENT input, so its digest MUST differ.
NEG_BREEDS="bayes,strips,frame"

# ── temp artifacts (idempotent cleanup via trap) ─────────────────────────────
TMPDIR_REPRO="$(mktemp -d /tmp/tpot2-repro-XXXXXX)"
cleanup() { rm -rf "$TMPDIR_REPRO"; }
trap cleanup EXIT

refuse() { echo "REPRO: REFUSED — $*" >&2; exit 1; }

# ── 1. build or locate the CLI binary (mirrors test_tpot2_e2e.sh) ────────────
# Preferred: build in-tree. Where the workspace cannot resolve (an isolated
# worktree whose siblings are not present), fall back to a pre-built binary on
# PATH or in a target/ dir. Locating a binary is an accepted path; fabricating
# output is not.
CLI=""
if cargo build -p lsp-max-cli >/dev/null 2>"$TMPDIR_REPRO/build.err"; then
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
  echo "REPRO: BLOCKED — could not build or locate lsp-max-cli binary" >&2
  [ -s "$TMPDIR_REPRO/build.err" ] && tail -5 "$TMPDIR_REPRO/build.err" >&2
  exit 2
fi
echo "REPRO: using CLI at $CLI"

run() { "$CLI" "$@"; }

# Project the load-bearing, reproducibility-relevant fields of a search result:
# the selected breed sequence, the best fitness, and the evaluation count. These
# are exactly the fields a fixed-seed search must reproduce bit-for-bit. The
# `summary` string is intentionally excluded only if it carried a wall-clock or
# nonce — here it does not, but we pin the determinism contract to the fields the
# search algorithm itself produces.
search_fingerprint() {
  jq -S '{
    breeds: (.best_pipeline.breeds // []),
    best_fitness: .best_fitness,
    evaluations: .evaluations,
    generations_run: .generations_run,
    status: .status
  }'
}

# Project the determinism-relevant fields of an evaluate result.
evaluate_fingerprint() {
  jq -S '{
    fitness: .pipeline.fitness,
    status: .status,
    breeds: (.pipeline.breeds // [])
  }'
}

# ── WITNESS 1: search determinism through the binary ─────────────────────────
# Two invocations with IDENTICAL args. The fixed internal seed means the
# normalized result MUST be byte-identical. A difference is a determinism break.
run pipeline search --generations "$GENERATIONS" --population-size "$POPULATION" \
  | search_fingerprint > "$TMPDIR_REPRO/search_a.json" \
  || refuse "first search invocation failed"
run pipeline search --generations "$GENERATIONS" --population-size "$POPULATION" \
  | search_fingerprint > "$TMPDIR_REPRO/search_b.json" \
  || refuse "second search invocation failed"

if ! cmp -s "$TMPDIR_REPRO/search_a.json" "$TMPDIR_REPRO/search_b.json"; then
  echo "REPRO: REFUSED — search NOT reproducible: identical args produced differing output" >&2
  diff -u "$TMPDIR_REPRO/search_a.json" "$TMPDIR_REPRO/search_b.json" >&2 || true
  exit 1
fi
W1_BREEDS="$(jq -r '.breeds | join(",")' "$TMPDIR_REPRO/search_a.json")"
W1_FITNESS="$(jq -r '.best_fitness' "$TMPDIR_REPRO/search_a.json")"
W1_STATUS="$(jq -r '.status' "$TMPDIR_REPRO/search_a.json")"
W1_EVALS="$(jq -r '.evaluations' "$TMPDIR_REPRO/search_a.json")"
[ -n "$W1_BREEDS" ] || refuse "search produced no best_pipeline breeds to bind"
echo "REPRO: WITNESS-1 ADMITTED — search byte-identical across 2 runs" \
     "(breeds=$W1_BREEDS fitness=$W1_FITNESS evaluations=$W1_EVALS status=$W1_STATUS)"

# ── WITNESS 2: evaluate determinism through the binary ───────────────────────
# The heuristic evaluator is a pure categorical function — same breeds, same
# fitness + status, every time.
run pipeline evaluate --breeds "$BREEDS" \
  | evaluate_fingerprint > "$TMPDIR_REPRO/eval_a.json" \
  || refuse "first evaluate invocation failed"
run pipeline evaluate --breeds "$BREEDS" \
  | evaluate_fingerprint > "$TMPDIR_REPRO/eval_b.json" \
  || refuse "second evaluate invocation failed"

if ! cmp -s "$TMPDIR_REPRO/eval_a.json" "$TMPDIR_REPRO/eval_b.json"; then
  echo "REPRO: REFUSED — evaluate NOT reproducible: identical breeds produced differing fitness/status" >&2
  diff -u "$TMPDIR_REPRO/eval_a.json" "$TMPDIR_REPRO/eval_b.json" >&2 || true
  exit 1
fi
EVAL_FITNESS="$(jq -r '.fitness' "$TMPDIR_REPRO/eval_a.json")"
EVAL_STATUS="$(jq -r '.status' "$TMPDIR_REPRO/eval_a.json")"
printf '%s' "$EVAL_STATUS" | grep -qE '^(ADMITTED|PARTIAL|UNKNOWN|REFUSED)$' \
  || refuse "evaluate status '$EVAL_STATUS' is not a bounded status"
echo "REPRO: WITNESS-2 ADMITTED — evaluate byte-identical across 2 runs" \
     "(breeds=$BREEDS fitness=$EVAL_FITNESS status=$EVAL_STATUS)"

# ── WITNESS 3: receipt digest stability for identical inputs ─────────────────
# Emit a receipt for the SAME witness-1 search result TWICE. The receipt digest
# is SHA256(breeds|fitness|ocel_path); identical inputs MUST yield an identical
# digest. The receipt is the ARTIFACT — this stdout is not.
RECEIPT_1="$TMPDIR_REPRO/repro_1.receipt.json"
RECEIPT_2="$TMPDIR_REPRO/repro_2.receipt.json"
bash scripts/pipeline-receipt.sh "$W1_BREEDS" "$W1_FITNESS" "$W1_STATUS" > "$RECEIPT_1" \
  || refuse "pipeline-receipt.sh failed to emit the first receipt"
bash scripts/pipeline-receipt.sh "$W1_BREEDS" "$W1_FITNESS" "$W1_STATUS" > "$RECEIPT_2" \
  || refuse "pipeline-receipt.sh failed to emit the second receipt"

DIGEST_1="$(jq -r '.digest' "$RECEIPT_1")"
DIGEST_2="$(jq -r '.digest' "$RECEIPT_2")"
printf '%s' "$DIGEST_1" | grep -qE '^[0-9a-f]{64}$' \
  || refuse "receipt-1 digest is not a 64-hex value: '$DIGEST_1'"
if [ "$DIGEST_1" != "$DIGEST_2" ]; then
  refuse "receipt digest NOT stable: identical inputs produced differing digests ($DIGEST_1 vs $DIGEST_2)"
fi
echo "REPRO: WITNESS-3 ADMITTED — receipt digest stable across identical inputs (digest=$DIGEST_1)"

# Validate one of the stable receipts via the chain validator.
val_out="$(bash scripts/validate-receipt-chain.sh "$RECEIPT_1")"
case "$val_out" in
  ADMITTED*) echo "REPRO: receipt validated — $val_out" ;;
  *) refuse "validate-receipt-chain did not ADMIT the stable receipt: $val_out" ;;
esac

# ── NEGATIVE CONTROL: a DIFFERENT input must produce a DIFFERENT digest ───────
# Emit a receipt for a different breed set. Because the digest binds its inputs,
# its digest MUST differ from witness-3's. A match would prove the digest is
# constant (does not bind inputs) and the discrimination claim collapses.
NEG_RECEIPT="$TMPDIR_REPRO/neg.receipt.json"
bash scripts/pipeline-receipt.sh "$NEG_BREEDS" "$W1_FITNESS" "$W1_STATUS" > "$NEG_RECEIPT" \
  || refuse "pipeline-receipt.sh failed to emit the negative-control receipt"
NEG_DIGEST="$(jq -r '.digest' "$NEG_RECEIPT")"
printf '%s' "$NEG_DIGEST" | grep -qE '^[0-9a-f]{64}$' \
  || refuse "negative-control digest is not a 64-hex value: '$NEG_DIGEST'"
if [ "$NEG_DIGEST" = "$DIGEST_1" ]; then
  refuse "negative control FAILED to discriminate: different breeds produced the SAME digest ($NEG_DIGEST) — digest does not bind inputs"
fi
echo "REPRO: NEG-CONTROL ADMITTED — different input yields distinct digest" \
     "(neg_breeds=$NEG_BREEDS neg_digest=$NEG_DIGEST != witness_digest=$DIGEST_1)"

# ── final bounded-status line ────────────────────────────────────────────────
echo "REPRO: ADMITTED — search+evaluate deterministic through binary, receipt digest stable across identical inputs, neg-control different-input digest distinct"
exit 0
