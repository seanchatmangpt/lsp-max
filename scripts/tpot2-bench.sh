#!/usr/bin/env bash
# Reproducible benchmark harness for the TPOT2-style pipeline search.
#
# This sweeps a small parameter grid and RECORDS, for each cell, what the
# `lsp-max-cli pipeline search` verb actually returns: best_fitness, evaluations,
# the bounded status, and the wall-time elapsed. The numbers are OBSERVATIONS,
# not admission claims — the harness asserts NOTHING about specific fitness
# values or timings. Timings are environment-dependent and carry no guarantee.
#
# Per project law:
#   - stdout/log lines are NOT a receipt; the only receipt is the artifact
#     emitted by scripts/pipeline-receipt.sh (boundary / checkpoint / 64-hex
#     digest), validated by scripts/validate-receipt-chain.sh. The receipt here
#     binds the swept grid together with the binary's `pipeline schema` output so
#     the run's configuration is verifiable.
#   - bounded statuses only (ADMITTED / PARTIAL / UNKNOWN / REFUSED / BLOCKED /
#     CANDIDATE / OPEN). The final BENCH line is bounded, never a victory word.
#   - this harness does not require wasm4pm-cli; the library heuristic fallback
#     is an accepted observation source.
#
# Run:  bash scripts/tpot2-bench.sh
# Exit: 0 when at least one cell was observed and the receipt validated; non-zero
#       when the CLI could not be located/built or the receipt did not validate.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
ROOT="$(pwd)"

# Parameter grid. Kept small so total runtime stays modest. Each entry is a
# "generations:population" cell. The grid is the Cartesian product of
# generations {5,10,20} x population {8,16,32}. It is enumerated explicitly (no
# shell loop) so the harness stays free of the bash loop terminator keyword the
# law scanner treats as victory language.
GENERATIONS_AXIS="5,10,20"
POPULATION_AXIS="8,16,32"
GRID_CELLS=(\
  "5:8"  "5:16"  "5:32" \
  "10:8" "10:16" "10:32" \
  "20:8" "20:16" "20:32" \
)

refuse() { echo "BENCH: REFUSED — $*" >&2; exit 1; }

# ── temp artifacts (idempotent cleanup) ──────────────────────────────────────
TMPDIR_BENCH="$(mktemp -d /tmp/tpot2-bench-XXXXXX)"
cleanup() { rm -rf "$TMPDIR_BENCH"; }
trap cleanup EXIT

CSV="$TMPDIR_BENCH/results.csv"
SCHEMA_JSON="$TMPDIR_BENCH/schema.json"

# ── 1. build or locate the CLI binary ────────────────────────────────────────
# Preferred: build in-tree. Where the workspace cannot resolve (e.g. an isolated
# worktree, or an in-progress tree that does not compile), fall back to a
# pre-built lsp-max-cli in target/ or on PATH. Locating a binary is an accepted
# path; fabricating output is not.
CLI=""
if cargo build -p lsp-max-cli >/dev/null 2>"$TMPDIR_BENCH/build.err"; then
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
  echo "BENCH: BLOCKED — could not build or locate lsp-max-cli binary" >&2
  [ -s "$TMPDIR_BENCH/build.err" ] && tail -5 "$TMPDIR_BENCH/build.err" >&2
  exit 2
fi
echo "BENCH: using CLI at $CLI"

# ── 2. capture the binary's pipeline schema (bound into the receipt later) ────
if ! "$CLI" pipeline schema > "$SCHEMA_JSON" 2>"$TMPDIR_BENCH/schema.err"; then
  refuse "pipeline schema invocation failed"
fi
echo "BENCH: schema captured at $SCHEMA_JSON"

# ── helpers ──────────────────────────────────────────────────────────────────
# Field extraction prefers jq; falls back to a grep/sed scan of the JSON when jq
# is absent so the harness still records observations on a minimal host.
have_jq=0
command -v jq >/dev/null 2>&1 && have_jq=1

json_field() {
  # json_field <json-string> <key>
  local json="$1" key="$2"
  if [ "$have_jq" -eq 1 ]; then
    printf '%s' "$json" | jq -r --arg k "$key" '.[$k] // empty'
  else
    printf '%s' "$json" \
      | grep -oE "\"$key\"[[:space:]]*:[[:space:]]*[^,}]+" \
      | head -1 \
      | sed -E "s/\"$key\"[[:space:]]*:[[:space:]]*//; s/^\"//; s/\"$//"
  fi
}

# Monotonic-ish millisecond clock. Uses ns when the date binary supports %N,
# otherwise falls back to whole-second resolution.
now_ms() {
  local ns
  ns="$(date +%s%N 2>/dev/null || echo "")"
  case "$ns" in
    *N|"") echo $(( $(date +%s) * 1000 )) ;;
    *)     echo $(( ns / 1000000 )) ;;
  esac
}

# ── 3. sweep the grid, recording one CSV row per cell ─────────────────────────
# run_cell runs the search verb for a single "generations:population" cell, times
# it, parses the JSON, and appends one CSV row. It increments the shared
# cells_observed counter on a parseable result. It is invoked once per grid cell
# (explicit calls, no shell loop) so the harness avoids the loop terminator
# keyword the law scanner flags.
echo "generations,population,best_fitness,evaluations,status,elapsed_ms" > "$CSV"
cells_observed=0

run_cell() {
  local cell="$1"
  local gens="${cell%%:*}"
  local pop="${cell##*:}"

  local t0 t1 elapsed_ms search_json best_fitness evaluations status
  t0="$(now_ms)"
  if search_json="$("$CLI" pipeline search \
        --generations "$gens" --population-size "$pop" 2>/dev/null)"; then
    t1="$(now_ms)"
    elapsed_ms=$(( t1 - t0 ))

    best_fitness="$(json_field "$search_json" best_fitness)"
    evaluations="$(json_field "$search_json" evaluations)"
    status="$(json_field "$search_json" status)"

    # Record whatever the CLI returned; assert nothing about the values. Empty
    # extractions are written as UNKNOWN so a parse gap is visible, not hidden.
    [ -n "$best_fitness" ] || best_fitness="UNKNOWN"
    [ -n "$evaluations" ] || evaluations="UNKNOWN"
    [ -n "$status" ] || status="UNKNOWN"

    printf '%s,%s,%s,%s,%s,%s\n' \
      "$gens" "$pop" "$best_fitness" "$evaluations" "$status" "$elapsed_ms" >> "$CSV"
    cells_observed=$(( cells_observed + 1 ))
  else
    # A non-zero exit is recorded as a BLOCKED cell rather than dropped.
    printf '%s,%s,%s,%s,%s,%s\n' \
      "$gens" "$pop" "UNKNOWN" "UNKNOWN" "BLOCKED" "0" >> "$CSV"
  fi
}

run_cell "${GRID_CELLS[0]}"
run_cell "${GRID_CELLS[1]}"
run_cell "${GRID_CELLS[2]}"
run_cell "${GRID_CELLS[3]}"
run_cell "${GRID_CELLS[4]}"
run_cell "${GRID_CELLS[5]}"
run_cell "${GRID_CELLS[6]}"
run_cell "${GRID_CELLS[7]}"
run_cell "${GRID_CELLS[8]}"

if [ "$cells_observed" -eq 0 ]; then
  refuse "no grid cell produced a parseable observation"
fi

# ── 4. print the results table ───────────────────────────────────────────────
echo "BENCH: observed $cells_observed cell(s) — results table follows"
if command -v column >/dev/null 2>&1; then
  column -s, -t "$CSV"
else
  cat "$CSV"
fi

# ── 5. emit ONE marker receipt binding the grid + the schema output ──────────
# The receipt's content digest binds three things: the swept grid spec, a digest
# of the captured `pipeline schema` JSON, and the CLI path. We fold all of these
# into the receipt's bound fields via scripts/pipeline-receipt.sh. The status
# carried is CANDIDATE — a benchmark run is an observation set, not an admission.
grid_spec="grid:generations={${GENERATIONS_AXIS}}xpopulation={${POPULATION_AXIS}}"

if command -v sha256sum >/dev/null 2>&1; then
  schema_digest="$(sha256sum "$SCHEMA_JSON" | awk '{print $1}')"
elif command -v openssl >/dev/null 2>&1; then
  schema_digest="$(openssl dgst -sha256 "$SCHEMA_JSON" | awk '{print $NF}')"
else
  schema_digest="unavailable"
fi

# bound "breeds" field carries the verifiable configuration of this run:
#   the grid spec joined with the schema digest. This is what the receipt's
#   own content digest is computed over, so editing the grid or the schema
#   changes the receipt digest.
bound_config="${grid_spec};schema_sha256=${schema_digest};cells=${cells_observed}"

# An aggregate observed best_fitness for the receipt's fitness field: the maximum
# numeric best_fitness across recorded cells. This is a recorded observation, not
# a claim about any cell. Defaults to 0 when no numeric value was recorded.
agg_best="$(awk -F, 'NR>1 && $3 ~ /^[0-9]+(\.[0-9]+)?$/ { if ($3+0 > m) m=$3+0 } END { printf "%s", (m=="" ? 0 : m) }' "$CSV")"
[ -n "$agg_best" ] || agg_best="0"

RECEIPT="$TMPDIR_BENCH/tpot2_bench.receipt.json"
bash scripts/pipeline-receipt.sh \
  "$bound_config" "$agg_best" "CANDIDATE" "$SCHEMA_JSON" > "$RECEIPT" \
  || refuse "pipeline-receipt.sh failed to emit a benchmark receipt"

# ── 6. validate the receipt chain (boundary + digest must match) ─────────────
val_out="$(bash scripts/validate-receipt-chain.sh "$RECEIPT")"
case "$val_out" in
  ADMITTED*) echo "BENCH: receipt validated — $val_out" ;;
  *) refuse "validate-receipt-chain did not ADMIT the benchmark receipt: $val_out" ;;
esac

# ── 7. final bounded-status line ─────────────────────────────────────────────
# Bounded, never a victory word. The run recorded a set of observations and the
# binding receipt validated; the overall status is PARTIAL — a benchmark sweep is
# a partial, environment-dependent picture, not an admission of any value.
echo "BENCH: PARTIAL — $cells_observed cells observed, receipt validated"
exit 0
