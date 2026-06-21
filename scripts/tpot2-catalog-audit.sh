#!/usr/bin/env bash
# tpot2-catalog-audit.sh — breed-catalog drift detector (knowledge-hooks / layer 4).
#
# The in-repo catalog at src/pipeline/catalog.rs hard-codes a KNOWN_BREEDS slice that
# must track the actual breeds defined in wasm4pm-cognition. This audit compares the
# in-repo set against the real breed source directory and reports drift as a bounded
# status so the catalog cannot silently rot.
#
# Drift directions:
#   - in catalog but not in source  => "stale/removed upstream"
#   - in source but not in catalog  => "missing from catalog"
#
# Outcomes (bounded statuses only):
#   CATALOG: ADMITTED — in sync (N breeds)
#   CATALOG: PARTIAL  — <a> missing, <b> stale
#   CATALOG: UNKNOWN  — breed source absent (sibling ../wasm4pm not checked out)
#
# A missing breed source yields UNKNOWN. It is never collapsed into ADMITTED or REFUSED:
# absence of the source is a gap in observation, not evidence of sync or of drift.
#
# Exit code is 0 in every case. This is a report, not a gate, so it composes cleanly;
# callers branch on the bounded CATALOG: line, not on the exit status. (A future ANDON
# diagnostic TPOT2-CATALOG-DRIFT could consume the PARTIAL line — CANDIDATE.)

set -euo pipefail

# Resolve repo root from this script's location so the audit runs from anywhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

CATALOG_FILE="${REPO_ROOT}/src/pipeline/catalog.rs"
BREED_SRC_DIR="${REPO_ROOT}/../wasm4pm/crates/wasm4pm-cognition/src/breeds"

# Base names excluded from the breed set. These match how the catalog was derived:
# module wiring, dispatch/registration plumbing, a stray test script, and a non-breed
# .rs.backup that the *.rs glob already skips (listed here so the intent is explicit).
# One name per line; consumed by `grep -vxF -f` as a whole-line fixed-string filter.
EXCLUDE_NAMES="mod
dispatch
registration
bayesian_network_test_script"

if [[ ! -f "${CATALOG_FILE}" ]]; then
  echo "CATALOG: UNKNOWN — in-repo catalog absent (${CATALOG_FILE} not found)"
  exit 0
fi

CATALOG_SET_FILE="$(mktemp)"
SOURCE_SET_FILE="$(mktemp)"
trap 'rm -f "${CATALOG_SET_FILE}" "${SOURCE_SET_FILE}"' EXIT

# Extract the in-repo catalog set: the quoted string ids inside the KNOWN_BREEDS slice,
# bounded from the `pub static KNOWN_BREEDS` line to the closing `];`.
awk '/pub static KNOWN_BREEDS/{f=1} f{print} f&&/\];/{exit}' "${CATALOG_FILE}" \
  | grep -oE '"[a-zA-Z0-9_]+"' \
  | tr -d '"' \
  | sort -u > "${CATALOG_SET_FILE}"

CATALOG_COUNT="$(grep -c . "${CATALOG_SET_FILE}" || true)"

# If the breed source directory is absent, the source set is unobservable. Report UNKNOWN
# and stop here — absence must not be read as sync (ADMITTED) or as drift (REFUSED).
if [[ ! -d "${BREED_SRC_DIR}" ]]; then
  echo "CATALOG: UNKNOWN — breed source absent (sibling ../wasm4pm not checked out)"
  echo "  in-repo catalog: ${CATALOG_COUNT} breeds (${CATALOG_FILE})"
  echo "  expected source: ${BREED_SRC_DIR}"
  exit 0
fi

# Compute the breed set from the directory: every `<name>.rs` becomes breed id `<name>`,
# minus the excluded base names. The `*.rs` glob already skips `registration.rs.backup`
# and the support/ subdirectory (subdir entries are not matched by a non-recursive glob).
(
  cd "${BREED_SRC_DIR}"
  ls -1 ./*.rs 2>/dev/null \
    | sed -e 's#^\./##' -e 's#\.rs$##' \
    | grep -vxF -f <(printf '%s\n' "${EXCLUDE_NAMES}") \
    | sort -u
) > "${SOURCE_SET_FILE}"

SOURCE_COUNT="$(grep -c . "${SOURCE_SET_FILE}" || true)"

# Drift lists via set difference.
#   missing = in source, not in catalog  (comm -13)
#   stale   = in catalog, not in source  (comm -23)
MISSING="$(comm -13 "${CATALOG_SET_FILE}" "${SOURCE_SET_FILE}" || true)"
STALE="$(comm -23 "${CATALOG_SET_FILE}" "${SOURCE_SET_FILE}" || true)"

MISSING_COUNT=0
STALE_COUNT=0
[[ -n "${MISSING}" ]] && MISSING_COUNT="$(printf '%s\n' "${MISSING}" | grep -c .)"
[[ -n "${STALE}" ]] && STALE_COUNT="$(printf '%s\n' "${STALE}" | grep -c .)"

echo "breed source : ${BREED_SRC_DIR}"
echo "in-repo cat. : ${CATALOG_FILE}"
echo "catalog count: ${CATALOG_COUNT}"
echo "source count : ${SOURCE_COUNT}"
echo

echo "missing from catalog (in source, not catalogued): ${MISSING_COUNT}"
if [[ "${MISSING_COUNT}" -gt 0 ]]; then
  printf '%s\n' "${MISSING}" | sed 's/^/  + /'
fi

echo "stale/removed upstream (catalogued, not in source): ${STALE_COUNT}"
if [[ "${STALE_COUNT}" -gt 0 ]]; then
  printf '%s\n' "${STALE}" | sed 's/^/  - /'
fi

echo

if [[ "${MISSING_COUNT}" -eq 0 && "${STALE_COUNT}" -eq 0 ]]; then
  echo "CATALOG: ADMITTED — in sync (${CATALOG_COUNT} breeds)"
else
  echo "CATALOG: PARTIAL — ${MISSING_COUNT} missing, ${STALE_COUNT} stale"
fi

exit 0
