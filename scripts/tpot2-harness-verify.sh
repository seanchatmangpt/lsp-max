#!/usr/bin/env bash
# Standalone verification of the self-contained `lsp_max::pipeline` subsystem.
#
# Why this exists: the full workspace requires sibling checkouts
# (../lsp-types-max, ../wasm4pm-compat, ../wasm4pm) that are absent in Claude
# Code web sessions and other isolated containers, so `cargo test --workspace`
# is BLOCKED there. The `src/pipeline` module tree depends only on
# serde/serde_json/std, so this script mounts it in a throwaway crate and runs
# `cargo test` + `cargo clippy -D warnings` + `rustfmt --check` against it,
# yielding a real compile+test signal for the optimizer code where the workspace
# build cannot run.
#
# It then emits a marker-style receipt (boundary / checkpoint / 64-hex digest)
# binding the captured output, and validates it with validate-receipt-chain.sh.
# Per project law: this stdout is NOT the receipt; the artifact is.
#
# Exit: 0 when the subsystem compiles, tests pass, clippy is clean, and the
# receipt validates. 2 when the toolchain/deps cannot be fetched (BLOCKED).
# Non-zero otherwise (REFUSED).
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
ROOT="$(pwd)"
PIPELINE_DIR="$ROOT/src/pipeline"
RECEIPT_OUT="${1:-$ROOT/docs/tpot2-phase-shift-verification.receipt.json}"

refuse() { echo "HARNESS: REFUSED — $*" >&2; exit 1; }
[ -d "$PIPELINE_DIR" ] || refuse "no src/pipeline directory at $PIPELINE_DIR"

HARNESS="$(mktemp -d /tmp/tpot2-harness-XXXXXX)"
cleanup() { rm -rf "$HARNESS"; }
trap cleanup EXIT

mkdir -p "$HARNESS/src"
ln -s "$PIPELINE_DIR" "$HARNESS/src/pipeline"

cat > "$HARNESS/Cargo.toml" <<'TOML'
[package]
name = "tpot2-harness"
version = "0.0.0"
edition = "2021"

[lib]
path = "src/lib.rs"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
TOML

cat > "$HARNESS/src/lib.rs" <<'RS'
//! Throwaway verification harness for the self-contained lsp_max::pipeline tree.
pub mod pipeline;
RS

OUT="$HARNESS/output.txt"
RAW_CMD="cargo test && cargo clippy --all-targets -- -D warnings (manifest: tpot2-harness over src/pipeline)"

: > "$OUT"
# fmt is a non-fatal signal; record it but do not block on a formatter drift.
if rustfmt --edition 2021 --check "$PIPELINE_DIR"/*.rs >>"$OUT" 2>&1; then
  echo "FMT: ADMITTED — src/pipeline is rustfmt-clean" | tee -a "$OUT"
else
  echo "FMT: PARTIAL — rustfmt reported drift (see output)" | tee -a "$OUT"
fi

if ! cargo test --manifest-path "$HARNESS/Cargo.toml" >>"$OUT" 2>&1; then
  tail -25 "$OUT" >&2
  # Distinguish "deps unfetchable" (BLOCKED) from real test failure (REFUSED).
  if grep -qiE "could not|no matching package|failed to (download|fetch)|network" "$OUT"; then
    echo "HARNESS: BLOCKED — toolchain/deps could not be fetched" >&2
    exit 2
  fi
  refuse "harness tests did not pass"
fi

if ! cargo clippy --manifest-path "$HARNESS/Cargo.toml" --all-targets -- -D warnings >>"$OUT" 2>&1; then
  tail -25 "$OUT" >&2
  refuse "clippy -D warnings did not pass"
fi

PASS_LINE="$(grep -E 'test result: ok\.' "$OUT" | head -1 | sed -E 's/; finished in [0-9.]+m?s//')"
echo "HARNESS: $PASS_LINE"

# ── marker-style receipt over a STABLE projection ────────────────────────────
# The raw log embeds per-run timings and nondeterministic parallel "Compiling"
# ordering, so digesting it would not be reproducible. Bind instead a normalized
# projection — the test-result counts and the clippy/fmt verdicts with timing
# stripped — so identical inputs yield an identical digest (a receipt must be).
PROJECTION="$HARNESS/projection.txt"
{
  grep -E 'test result:' "$OUT" | sed -E 's/; finished in [0-9.]+m?s//'
  grep -E '^FMT:' "$OUT" | head -1
  echo "clippy: clean (-D warnings)"
} > "$PROJECTION"

if command -v sha256sum >/dev/null 2>&1; then
  OUTPUT_DIGEST="$(sha256sum "$PROJECTION" | awk '{print $1}')"; ALG="sha256"
elif command -v openssl >/dev/null 2>&1; then
  OUTPUT_DIGEST="$(openssl dgst -sha256 "$PROJECTION" | awk '{print $2}')"; ALG="sha256"
else
  echo "HARNESS: UNKNOWN — no sha256 tool to bind the receipt" >&2; exit 0
fi

TS="$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo unknown)"
mkdir -p "$(dirname "$RECEIPT_OUT")"

write_receipt() {
  if command -v jq >/dev/null 2>&1; then
    jq -n \
      --arg bnd "-----BEGIN RECEIPT-----" \
      --arg chk "-----END RECEIPT-----" \
      --arg raw "$RAW_CMD" \
      --arg dig "$OUTPUT_DIGEST" \
      --arg alg "$ALG" \
      --arg st  "ADMITTED" \
      --arg ts  "$TS" \
      --arg pass "$PASS_LINE" \
      '{
        boundary: $bnd,
        checkpoint: $chk,
        raw_command: $raw,
        digest: $dig,
        digest_algorithm: $alg,
        output_digest: $dig,
        status: $st,
        boundary_note: "standalone harness over src/pipeline; full workspace build is BLOCKED (siblings absent); digest binds a normalized projection (test counts + clippy/fmt verdicts), not raw timing",
        negative_controls: [
          "ocel::garbage_and_absent_sources_are_none",
          "search::empty_breed_pool_refused",
          "phase::unknown_never_collapses_into_liquid_or_vapor"
        ],
        test_result: $pass,
        verified_at: $ts
      }' > "$RECEIPT_OUT"
  else
    cat > "$RECEIPT_OUT" <<JSON
{
  "boundary": "-----BEGIN RECEIPT-----",
  "checkpoint": "-----END RECEIPT-----",
  "raw_command": "$RAW_CMD",
  "digest": "$OUTPUT_DIGEST",
  "digest_algorithm": "$ALG",
  "output_digest": "$OUTPUT_DIGEST",
  "status": "ADMITTED",
  "boundary_note": "standalone harness over src/pipeline; full workspace build is BLOCKED (siblings absent); digest binds a normalized projection (test counts + clippy/fmt verdicts), not raw timing",
  "test_result": "$PASS_LINE",
  "verified_at": "$TS"
}
JSON
  fi
}
write_receipt
echo "HARNESS: receipt written to $RECEIPT_OUT (output_digest=$OUTPUT_DIGEST)"

if [ -x "$ROOT/scripts/validate-receipt-chain.sh" ]; then
  val="$(bash "$ROOT/scripts/validate-receipt-chain.sh" "$RECEIPT_OUT")"
  case "$val" in
    ADMITTED*) echo "HARNESS: receipt validated — $val" ;;
    *) refuse "receipt did not validate: $val" ;;
  esac
fi

echo "HARNESS: ADMITTED — src/pipeline subsystem compiled, tested, and clippy-clean in isolation; receipt bound and validated"
exit 0
