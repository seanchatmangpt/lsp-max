#!/usr/bin/env bash
#
# scripts/doctor-repair.sh
#
# The self-healing arm of lsp-max. DETECTS the exact breakage classes this
# workspace recurrently hits and — only when invoked with --apply — REMEDIATES
# the subset of them that are provably safe (filesystem + git-index only),
# emitting a signed receipt artifact for every applied action.
#
# Law boundaries (AGENTS.md / CLAUDE.md):
#   - Read-only toward user source by default. Source code is NEVER auto-edited.
#   - Only safe filesystem / git-index actions run under --apply; everything that
#     would touch tracked source (conflict markers, manifest drift) is REPORT-only.
#   - A receipt artifact (path + digest + boundary markers + checkpoint + bounded
#     per-action outcome) is the only proof of action. Stdout is not a receipt.
#   - No victory language. Outcomes use bounded statuses only:
#       ADMITTED CANDIDATE BLOCKED REFUSED UNKNOWN PARTIAL OPEN
#
# Usage:
#   scripts/doctor-repair.sh            # print bounded-status REPAIR PLAN; no mutation
#   scripts/doctor-repair.sh --apply    # perform safe repairs + write receipts/<ts>.receipt.json
#   scripts/doctor-repair.sh --help
#
# Exit codes:
#   0  plan printed, or --apply ran and every applied action was ADMITTED
#   1  --apply ran but at least one applied action is BLOCKED/PARTIAL
#   2  usage error

set -euo pipefail

# ---------------------------------------------------------------------------
# Bounded-status palette (no victory words anywhere in this file)
# ---------------------------------------------------------------------------
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

APPLY=0
for arg in "$@"; do
  case "$arg" in
    --apply) APPLY=1 ;;
    -h|--help)
      sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo -e "${RED}REFUSED${NC}: unknown argument '$arg' (expected --apply or --help)" >&2
      exit 2
      ;;
  esac
done

# Disk-reclaim threshold: remediate prunable build dirs only when free space on
# the workspace filesystem is at or below this many MiB.
LOW_DISK_MIB="${DOCTOR_LOW_DISK_MIB:-2048}"

# Workspace version drives the checkpoint label. Read it, never hardcode a date.
WORKSPACE_VERSION="$(
  awk '/^\[workspace.package\]/{f=1} f&&/^version[[:space:]]*=/{gsub(/[",]/,"",$3); print $3; exit}' \
    "$PROJECT_ROOT/Cargo.toml" 2>/dev/null || true
)"
WORKSPACE_VERSION="${WORKSPACE_VERSION:-UNKNOWN}"

# ---------------------------------------------------------------------------
# Plan accumulation. Each finding is one line: STATUS<TAB>CLASS<TAB>detail
# ---------------------------------------------------------------------------
PLAN_LINES=()
plan_add() { PLAN_LINES+=("$1	$2	$3"); }

# Applied-action ledger (only populated under --apply). One line per action:
# STATUS<TAB>ACTION<TAB>detail
APPLIED_LINES=()
applied_add() { APPLIED_LINES+=("$1	$2	$3"); }

have() { command -v "$1" >/dev/null 2>&1; }
in_git_repo() { git rev-parse --is-inside-work-tree >/dev/null 2>&1; }

# ===========================================================================
# DETECTOR 1 — Low disk: prunable build artifacts (target/doc, debug/incremental)
# ===========================================================================
DISK_FREE_MIB="UNKNOWN"
detect_disk() {
  if have df; then
    # POSIX df in 1K blocks; column 4 = available.
    local avail_kib
    avail_kib="$(df -Pk "$PROJECT_ROOT" 2>/dev/null | awk 'NR==2{print $4}')"
    if [[ -n "${avail_kib:-}" ]]; then
      DISK_FREE_MIB=$(( avail_kib / 1024 ))
    fi
  fi

  local prune_targets=("target/doc" "target/debug/incremental")
  local present=()
  for d in "${prune_targets[@]}"; do
    [[ -d "$PROJECT_ROOT/$d" ]] && present+=("$d")
  done

  if [[ "$DISK_FREE_MIB" == "UNKNOWN" ]]; then
    plan_add UNKNOWN DISK-RECLAIM "free space UNKNOWN (df unavailable); prunable dirs present: ${present[*]:-none}"
    return
  fi

  if (( DISK_FREE_MIB > LOW_DISK_MIB )); then
    plan_add OPEN DISK-RECLAIM "free ${DISK_FREE_MIB}MiB > ${LOW_DISK_MIB}MiB threshold; no reclaim needed"
    return
  fi

  if (( ${#present[@]} == 0 )); then
    plan_add OPEN DISK-RECLAIM "free ${DISK_FREE_MIB}MiB <= ${LOW_DISK_MIB}MiB but no prunable build dirs exist"
    return
  fi

  for d in "${present[@]}"; do
    plan_add CANDIDATE DISK-RECLAIM "free ${DISK_FREE_MIB}MiB <= ${LOW_DISK_MIB}MiB; prune '$d' (regenerable build output)"
  done
}

apply_disk() {
  local did_any=0
  for line in "${PLAN_LINES[@]}"; do
    IFS=$'\t' read -r status class detail <<<"$line"
    [[ "$class" == "DISK-RECLAIM" && "$status" == "CANDIDATE" ]] || continue
    # detail ends with: prune '<dir>' (...)
    local dir
    dir="$(printf '%s' "$detail" | sed -n "s/.*prune '\([^']*\)'.*/\1/p")"
    [[ -n "$dir" && -d "$PROJECT_ROOT/$dir" ]] || { applied_add UNKNOWN DISK-RECLAIM "target '$dir' vanished before apply"; continue; }
    if rm -rf -- "${PROJECT_ROOT:?}/$dir"; then
      applied_add ADMITTED DISK-RECLAIM "pruned regenerable build dir '$dir'"
    else
      applied_add BLOCKED DISK-RECLAIM "rm failed for '$dir'"
    fi
    did_any=1
  done
  if (( did_any == 0 )); then
    applied_add OPEN DISK-RECLAIM "no disk-reclaim candidate in plan"
  fi
}

# ===========================================================================
# DETECTOR 2 — Tracked-but-gitignored runtime artifacts (untrack via git rm --cached)
# Never deletes working files; only removes them from the git index.
# ===========================================================================
TRACKED_IGNORED=()
detect_tracked_ignored() {
  if ! in_git_repo; then
    plan_add UNKNOWN UNTRACK-IGNORED "not inside a git work tree; cannot inspect index"
    return
  fi
  # git ls-files ∩ (gitignore rules). -z keeps paths with spaces intact.
  # --no-index is essential: git check-ignore normally reports a path as ignored
  # only when it is NOT already tracked. The breakage we hunt is exactly a file
  # that is BOTH tracked AND matched by a .gitignore rule, so the index state
  # must be excluded from the ignore evaluation.
  local f
  while IFS= read -r -d '' f; do
    [[ -n "$f" ]] || continue
    if git check-ignore -q --no-index -- "$f" 2>/dev/null; then
      TRACKED_IGNORED+=("$f")
    fi
  done < <(git ls-files -z 2>/dev/null || true)

  if (( ${#TRACKED_IGNORED[@]} == 0 )); then
    plan_add OPEN UNTRACK-IGNORED "no tracked file matches .gitignore"
    return
  fi
  for f in "${TRACKED_IGNORED[@]}"; do
    plan_add CANDIDATE UNTRACK-IGNORED "tracked-but-ignored: git rm --cached -- '$f' (working file kept)"
  done
}

apply_tracked_ignored() {
  if (( ${#TRACKED_IGNORED[@]} == 0 )); then
    applied_add OPEN UNTRACK-IGNORED "no tracked-but-ignored path to untrack"
    return
  fi
  local f
  for f in "${TRACKED_IGNORED[@]}"; do
    if [[ ! -e "$f" ]]; then
      applied_add UNKNOWN UNTRACK-IGNORED "path '$f' vanished before apply"
      continue
    fi
    # --cached: index-only; the working-tree file is preserved.
    if git rm --cached --quiet -- "$f" 2>/dev/null; then
      applied_add ADMITTED UNTRACK-IGNORED "untracked '$f' from index (working file preserved)"
    else
      applied_add BLOCKED UNTRACK-IGNORED "git rm --cached failed for '$f'"
    fi
  done
}

# ===========================================================================
# DETECTOR 3 — Committed merge-conflict markers in tracked source (REPORT ONLY)
# Source is never auto-edited. Reports file:line only.
# ===========================================================================
detect_conflict_markers() {
  if ! in_git_repo; then
    plan_add UNKNOWN CONFLICT-MARKER "not inside a git work tree; cannot scan tracked files"
    return
  fi
  local scanner="" hits=""
  if have rg; then
    scanner="rg"
    # ^<<<<<<< , ^======= , ^>>>>>>>  — anchored conflict sigils on tracked files.
    hits="$(git ls-files -z 2>/dev/null \
      | xargs -0 rg -n --no-heading -e '^<{7}( |$)' -e '^={7}$' -e '^>{7}( |$)' -- 2>/dev/null || true)"
  else
    scanner="grep"
    hits="$(git ls-files -z 2>/dev/null \
      | xargs -0 grep -nE '^(<{7}( |$)|={7}$|>{7}( |$))' -- 2>/dev/null || true)"
  fi

  if [[ -z "$hits" ]]; then
    plan_add OPEN CONFLICT-MARKER "no committed conflict markers in tracked source (scanner=$scanner)"
    return
  fi
  # REFUSED to auto-edit; reported for a human. file:line carried verbatim.
  while IFS= read -r h; do
    [[ -n "$h" ]] || continue
    plan_add REFUSED CONFLICT-MARKER "merge-conflict marker (manual edit required): $h"
  done <<<"$hits"
}

# ===========================================================================
# DETECTOR 4 — Manifest path/version drift (REPORT ONLY; manifests need review)
# Two coupled bugs from a recent migration:
#   (a) a path dep's `version = "X"` disagrees with the referenced crate's actual
#       `version` in its own Cargo.toml.
#   (b) a path dep's relative `path = "..."` does not resolve to a Cargo.toml at
#       the stated depth.
# Reports precise file:line plus the correct value. Never auto-edits manifests.
# ===========================================================================
detect_manifest_drift() {
  local found_any=0 manifest
  local -a manifests=()
  # NUL-safe enumeration: git index first, ripgrep fallback. Neither path lets a
  # NUL byte enter a shell string variable (which bash warns about).
  if in_git_repo; then
    while IFS= read -r -d '' manifest; do
      manifests+=("$manifest")
    done < <(git ls-files -z '*Cargo.toml' 2>/dev/null || true)
  fi
  if (( ${#manifests[@]} == 0 )) && have rg; then
    while IFS= read -r -d '' manifest; do
      manifests+=("$manifest")
    done < <(rg --files -g 'Cargo.toml' --glob '!target/**' -0 2>/dev/null || true)
  fi
  if (( ${#manifests[@]} == 0 )); then
    plan_add UNKNOWN MANIFEST-DRIFT "no Cargo.toml manifests enumerable"
    return
  fi

  for manifest in "${manifests[@]}"; do
    [[ -f "$manifest" ]] || continue
    local manifest_dir
    manifest_dir="$(cd "$(dirname "$manifest")" && pwd)"

    # Match single-line table deps that carry both `path =` and (optionally)
    # `version =`. Multi-line dep tables are reported as UNKNOWN (not parsed here).
    local lineno=0 raw
    while IFS= read -r raw; do
      lineno=$((lineno + 1))
      printf '%s' "$raw" | grep -qE 'path[[:space:]]*=' || continue
      printf '%s' "$raw" | grep -qE '^\s*[A-Za-z0-9_-]+\s*=\s*\{' || continue

      local dep_name rel_path
      dep_name="$(printf '%s' "$raw" | sed -n 's/^[[:space:]]*\([A-Za-z0-9_-]\+\)[[:space:]]*=.*/\1/p')"
      rel_path="$(printf '%s' "$raw" | sed -n 's/.*path[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p')"
      [[ -n "$rel_path" ]] || continue

      # --- (b) path-depth resolution ---
      local target_manifest="$manifest_dir/$rel_path/Cargo.toml"
      local resolved=""
      if [[ -f "$target_manifest" ]]; then
        resolved="$(cd "$(dirname "$target_manifest")" && pwd)/Cargo.toml"
      fi
      if [[ -z "$resolved" ]]; then
        local rel_disp="${manifest#"$PROJECT_ROOT"/}"
        plan_add REFUSED MANIFEST-DRIFT \
          "${rel_disp}:${lineno}: dep '${dep_name}' path='${rel_path}' does not resolve to a Cargo.toml from ${manifest_dir} (path-depth drift; fix path, manual review)"
        found_any=1
        continue
      fi

      # --- (a) version drift (only when the line declares a version) ---
      local declared_version actual_version
      declared_version="$(printf '%s' "$raw" | sed -n 's/.*version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p')"
      if [[ -n "$declared_version" ]]; then
        actual_version="$(
          awk '/^\[package\]/{p=1} p&&/^version[[:space:]]*=/{gsub(/[",]/,"",$3); print $3; exit}' \
            "$resolved" 2>/dev/null || true
        )"
        if [[ -z "$actual_version" ]]; then
          local rel_disp="${manifest#"$PROJECT_ROOT"/}"
          plan_add UNKNOWN MANIFEST-DRIFT \
            "${rel_disp}:${lineno}: dep '${dep_name}' version='${declared_version}' but referenced ${resolved#"$PROJECT_ROOT"/} has no [package].version (cannot compare)"
          found_any=1
        elif [[ "$declared_version" != "$actual_version" ]]; then
          local rel_disp="${manifest#"$PROJECT_ROOT"/}"
          plan_add REFUSED MANIFEST-DRIFT \
            "${rel_disp}:${lineno}: dep '${dep_name}' declares version='${declared_version}' but ${resolved#"$PROJECT_ROOT"/} is version='${actual_version}' — correct value: '${actual_version}' (version drift, manual review)"
          found_any=1
        fi
      fi
    done <"$manifest"
  done

  if (( found_any == 0 )); then
    plan_add OPEN MANIFEST-DRIFT "no single-line path-dep version/path drift detected in tracked manifests"
  fi
}

# ===========================================================================
# Receipt emission (only under --apply).
#
# Two artifacts are written per run, mirroring the split used by the in-repo
# receipt scripts (write_bench_receipt.sh / generate-lsp-receipts.sh):
#
#   <run_id>.receipt.json  — PURE JSON (parseable by serde_json::from_str, the
#                            same shape the receipt tests assert). It carries
#                            `boundary` and `checkpoint` as string fields whose
#                            values are the canonical markers, plus digest,
#                            digest_algorithm, raw_command, status.
#   <run_id>.body.txt      — the digest-bound applied-action ledger, wrapped in
#                            literal -----BEGIN RECEIPT----- / -----END RECEIPT-----
#                            lines and a `Checkpoint:` line, so a text-scanning
#                            chain validator finds the markers AND the digest in
#                            the JSON provably binds them.
# ===========================================================================
digest_file() {
  # Prefer blake3 if present, else sha256. Echoes "<alg> <hexdigest>".
  local f="$1"
  if have b3sum; then
    printf 'BLAKE3 %s' "$(b3sum --no-names "$f" 2>/dev/null | awk '{print $1}')"
  elif have blake3; then
    printf 'BLAKE3 %s' "$(blake3 "$f" 2>/dev/null | awk '{print $1}')"
  elif have sha256sum; then
    printf 'SHA256 %s' "$(sha256sum "$f" 2>/dev/null | awk '{print $1}')"
  elif have shasum; then
    printf 'SHA256 %s' "$(shasum -a 256 "$f" 2>/dev/null | awk '{print $1}')"
  else
    printf 'UNKNOWN %s' "0000000000000000000000000000000000000000000000000000000000000000"
  fi
}

write_receipt() {
  local receipts_dir="$PROJECT_ROOT/receipts"
  mkdir -p "$receipts_dir"
  local ts iso run_id
  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  run_id="doctor-repair-${ts}"
  local receipt_path="$receipts_dir/${run_id}.receipt.json"

  local checkpoint="DOCTOR-REPAIR-${WORKSPACE_VERSION}-${ts}"

  # Body sidecar = the artifact the digest is taken over. The applied-action
  # ledger, wrapped in the canonical boundary markers and a Checkpoint line, so
  # the digest binds the exact outcomes AND a text-scanning validator finds the
  # markers here.
  local body_file="$receipts_dir/${run_id}.body.txt"
  local overall="ADMITTED" line status action detail
  {
    echo "-----BEGIN RECEIPT-----"
    echo "Checkpoint: $checkpoint"
    for line in "${APPLIED_LINES[@]}"; do
      IFS=$'\t' read -r status action detail <<<"$line"
      printf '%s\t%s\t%s\n' "$status" "$action" "$detail"
    done
    echo "-----END RECEIPT-----"
  } >"$body_file"
  # Overall verdict (separate pass; bounded, never victorious).
  for line in "${APPLIED_LINES[@]}"; do
    IFS=$'\t' read -r status action detail <<<"$line"
    case "$status" in
      BLOCKED|PARTIAL) overall="PARTIAL" ;;
    esac
  done

  local alg digest pair
  pair="$(digest_file "$body_file")"
  alg="${pair%% *}"
  digest="${pair#* }"

  # Build the JSON actions array from the ledger.
  local actions_json="" first=1
  for line in "${APPLIED_LINES[@]}"; do
    IFS=$'\t' read -r status action detail <<<"$line"
    # JSON-escape the detail string (backslash, quote, control chars).
    local esc
    esc="$(printf '%s' "$detail" | sed 's/\\/\\\\/g; s/"/\\"/g')"
    [[ $first -eq 0 ]] && actions_json+=","
    actions_json+=$(printf '\n    {"status": "%s", "action": "%s", "detail": "%s"}' "$status" "$action" "$esc")
    first=0
  done

  # Pure JSON: parseable by serde_json::from_str, matching the in-repo receipt
  # convention. `boundary`/`checkpoint` carry the canonical marker text; the
  # digest-bound, marker-wrapped artifact lives at `body_path`.
  local esc_body
  esc_body="$(printf '%s' "$body_file" | sed 's/\\/\\\\/g; s/"/\\"/g')"
  cat >"$receipt_path" <<EOF
{
  "checkpoint": "$checkpoint",
  "boundary": "-----BEGIN RECEIPT-----",
  "checkpoint_close": "-----END RECEIPT-----",
  "body_path": "$esc_body",
  "digest": "$digest",
  "digest_algorithm": "$alg",
  "raw_command": "scripts/doctor-repair.sh --apply",
  "producing_workspace": "lsp-max",
  "workspace_version": "$WORKSPACE_VERSION",
  "timestamp": "$iso",
  "run_id": "$run_id",
  "actions": [$actions_json
  ],
  "status": "$overall"
}
EOF

  echo "$receipt_path	$overall	$alg	$digest"
}

# ===========================================================================
# Run detectors
# ===========================================================================
detect_disk
detect_tracked_ignored
detect_conflict_markers
detect_manifest_drift

print_plan() {
  echo -e "${BLUE}========================================${NC}"
  echo -e "${BLUE} lsp-max doctor-repair — REPAIR PLAN${NC}"
  echo -e "${BLUE}========================================${NC}"
  echo -e "workspace_version: ${WORKSPACE_VERSION}   free_disk: ${DISK_FREE_MIB}MiB   low_threshold: ${LOW_DISK_MIB}MiB"
  echo -e "mode: $([[ $APPLY -eq 1 ]] && echo 'APPLY (safe fs/git-index actions only)' || echo 'PLAN (no mutation)')"
  echo ""
  local line status class detail color
  for line in "${PLAN_LINES[@]}"; do
    IFS=$'\t' read -r status class detail <<<"$line"
    case "$status" in
      ADMITTED|OPEN) color="$GREEN" ;;
      CANDIDATE)     color="$YELLOW" ;;
      REFUSED|BLOCKED) color="$RED" ;;
      *)             color="$NC" ;;
    esac
    printf "%b%-9s%b %-16s %s\n" "$color" "$status" "$NC" "$class" "$detail"
  done
  echo ""
  echo -e "${BLUE}Legend:${NC} CANDIDATE=safe repair available under --apply | REFUSED=manual review (source/manifest, never auto-edited) | OPEN=nothing to repair | UNKNOWN=undetermined"
}

print_plan

if (( APPLY == 0 )); then
  # PLAN mode: no mutation, no receipt. CANDIDATEs indicate repairs are available.
  echo ""
  echo -e "${YELLOW}PLAN only.${NC} Re-run with --apply to perform CANDIDATE filesystem/git-index repairs and emit a receipt."
  exit 0
fi

# ---------------------------------------------------------------------------
# APPLY mode: perform only the safe (fs + git-index) repairs, then write receipt.
# ---------------------------------------------------------------------------
echo ""
echo -e "${BLUE}----------------------------------------${NC}"
echo -e "${BLUE} Applying safe repairs (fs + git-index)${NC}"
echo -e "${BLUE}----------------------------------------${NC}"

apply_disk
apply_tracked_ignored
# CONFLICT-MARKER and MANIFEST-DRIFT are REPORT-only: no apply path by law.
applied_add REFUSED CONFLICT-MARKER "source conflict markers are never auto-edited (see plan)"
applied_add REFUSED MANIFEST-DRIFT "manifest path/version edits require human review (see plan)"

EXIT=0
for line in "${APPLIED_LINES[@]}"; do
  IFS=$'\t' read -r status action detail <<<"$line"
  case "$status" in
    ADMITTED) printf "%bADMITTED %b %-16s %s\n" "$GREEN" "$NC" "$action" "$detail" ;;
    OPEN|REFUSED) printf "%-9s %-16s %s\n" "$status" "$action" "$detail" ;;
    BLOCKED|PARTIAL) printf "%b%-9s%b %-16s %s\n" "$RED" "$status" "$NC" "$action" "$detail"; EXIT=1 ;;
    *) printf "%-9s %-16s %s\n" "$status" "$action" "$detail" ;;
  esac
done

echo ""
RECEIPT_LINE="$(write_receipt)"
IFS=$'\t' read -r r_path r_status r_alg r_digest <<<"$RECEIPT_LINE" || true
echo -e "${BLUE}Receipt written:${NC} $r_path"
echo -e "  status=${r_status}  digest_algorithm=${r_alg}  digest=${r_digest}"
echo -e "  Validate with: scripts/validate-receipt-chain.sh $r_path"

exit $EXIT
