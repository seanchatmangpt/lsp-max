#!/bin/bash
#
# scripts/qol-budget.sh
#
# AutoQoL: target-dir budget guard.
#
# This workspace plus three sibling repos emit heavy build artifacts; a single
# `cargo test --workspace` has filled the disk in one session. This guard
# measures the on-disk weight of `target/` and the free space on its
# filesystem, then proposes a bounded reclamation plan.
#
# Reclamation tiers (least to most destructive):
#   SOFT  (over soft threshold) — prune target/doc, target/debug/incremental,
#                                  and obvious stale artifacts (*.rmeta orphans
#                                  under deps are NOT touched; only doc +
#                                  incremental + fingerprint scratch).
#   HARD  (over hard cap)       — escalate to `cargo clean`, but ONLY when
#                                  `--apply` is also passed. Never automatic.
#
# Default is a DRY-RUN: it prints what it WOULD reclaim and exits without
# mutating the tree. Pass `--apply` to actually prune.
#
# Output uses bounded statuses only (ADMITTED / CANDIDATE / BLOCKED /
# PARTIAL / OPEN / UNKNOWN). No victory language.
#
# Exit codes:
#   0 = under soft threshold, or prune carried out / planned without error
#   2 = over hard cap (escalation indicated) — surfaced to caller as ANDON-ish
#       pressure signal; not an error of this script itself
#
# Usage:
#   scripts/qol-budget.sh                 # dry-run plan
#   scripts/qol-budget.sh --apply         # carry out soft prune (+ hard clean if past cap)
#   scripts/qol-budget.sh --soft 8000     # soft threshold in MB (default 8000)
#   scripts/qol-budget.sh --hard 16000    # hard cap in MB (default 16000)
#   scripts/qol-budget.sh --target DIR    # target dir (default <root>/target)

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# ----------------------------------------------------------------------------
# Argument parsing
# ----------------------------------------------------------------------------
APPLY=0
SOFT_MB=8000
HARD_MB=16000
TARGET_DIR="$PROJECT_ROOT/target"

while [ $# -gt 0 ]; do
  case "$1" in
    --apply) APPLY=1; shift ;;
    --soft) SOFT_MB="${2:?--soft needs an MB value}"; shift 2 ;;
    --hard) HARD_MB="${2:?--hard needs an MB value}"; shift 2 ;;
    --target) TARGET_DIR="${2:?--target needs a path}"; shift 2 ;;
    -h|--help)
      grep '^#' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo -e "${RED}UNKNOWN argument: $1${NC}" >&2
      exit 1
      ;;
  esac
done

echo -e "${MAGENTA}========================================${NC}"
echo -e "${CYAN} AutoQoL: Target Budget Guard${NC}"
echo -e "${MAGENTA}========================================${NC}"
if [ "$APPLY" -eq 1 ]; then
  echo -e "${YELLOW}mode: --apply (mutating prune authorized)${NC}"
else
  echo -e "${BLUE}mode: dry-run (plan only; pass --apply to prune)${NC}"
fi
echo -e "${BLUE}target: ${TARGET_DIR}${NC}"
echo -e "${BLUE}soft threshold: ${SOFT_MB} MB   hard cap: ${HARD_MB} MB${NC}"

# ----------------------------------------------------------------------------
# Helpers
# ----------------------------------------------------------------------------

# Size of a path in MB (integer). Missing path => 0.
dir_mb() {
  local path="$1"
  if [ -e "$path" ]; then
    du -sm "$path" 2>/dev/null | awk '{print $1}' || echo 0
  else
    echo 0
  fi
}

# Human-readable MB.
fmt_mb() { printf '%s MB' "$1"; }

# ----------------------------------------------------------------------------
# Measurement
# ----------------------------------------------------------------------------
echo -e "\n${BLUE}[1/3] Measuring on-disk weight...${NC}"

if [ ! -d "$TARGET_DIR" ]; then
  echo -e "${YELLOW}target dir absent — nothing built yet.${NC}"
  echo -e "Budget status:        ${GREEN}ADMITTED${NC} (0 MB, under threshold)"
  exit 0
fi

TARGET_TOTAL_MB="$(dir_mb "$TARGET_DIR")"

# Free space on the filesystem that holds the target dir, in MB.
FREE_MB="$(df -Pm "$TARGET_DIR" 2>/dev/null | awk 'NR==2 {print $4}')"
FREE_MB="${FREE_MB:-0}"

# Prune-candidate sub-paths and their individual weights.
DOC_DIR="$TARGET_DIR/doc"
INCR_DIRS=()
# Incremental caches live under each profile dir (debug, ci, release, ...).
while IFS= read -r d; do
  [ -n "$d" ] && INCR_DIRS+=("$d")
done < <(find "$TARGET_DIR" -maxdepth 2 -type d -name incremental 2>/dev/null || true)

DOC_MB="$(dir_mb "$DOC_DIR")"

INCR_MB=0
for d in "${INCR_DIRS[@]:-}"; do
  [ -n "${d:-}" ] || continue
  sz="$(dir_mb "$d")"
  INCR_MB=$((INCR_MB + sz))
done

# Stale top-level scratch that is safe to drop without a rebuild of artifacts:
#   - target/tmp (transient)
#   - target/.rustc_info.json (regenerated on next invocation)
STALE_PATHS=()
[ -d "$TARGET_DIR/tmp" ] && STALE_PATHS+=("$TARGET_DIR/tmp")
[ -f "$TARGET_DIR/.rustc_info.json" ] && STALE_PATHS+=("$TARGET_DIR/.rustc_info.json")
STALE_MB=0
for p in "${STALE_PATHS[@]:-}"; do
  [ -n "${p:-}" ] || continue
  sz="$(dir_mb "$p")"
  STALE_MB=$((STALE_MB + sz))
done

SOFT_RECLAIM_MB=$((DOC_MB + INCR_MB + STALE_MB))

echo -e "  target total:        $(fmt_mb "$TARGET_TOTAL_MB")"
echo -e "  free on filesystem:  $(fmt_mb "$FREE_MB")"
echo -e "  reclaimable (soft):  $(fmt_mb "$SOFT_RECLAIM_MB")"
echo -e "    - target/doc:          $(fmt_mb "$DOC_MB")"
echo -e "    - incremental caches:  $(fmt_mb "$INCR_MB")"
echo -e "    - stale scratch:       $(fmt_mb "$STALE_MB")"

# ----------------------------------------------------------------------------
# Classification
# ----------------------------------------------------------------------------
echo -e "\n${BLUE}[2/3] Classifying budget pressure...${NC}"

# Tier decision is by target size; free space lowers thresholds defensively:
# if free space is critically low (< 2000 MB) we treat as hard pressure even
# when the target dir itself has not crossed the hard cap.
LOW_FREE_MB=2000

TIER="OPEN"
if [ "$TARGET_TOTAL_MB" -ge "$HARD_MB" ] || [ "$FREE_MB" -lt "$LOW_FREE_MB" ]; then
  TIER="HARD"
elif [ "$TARGET_TOTAL_MB" -ge "$SOFT_MB" ]; then
  TIER="SOFT"
else
  TIER="UNDER"
fi

case "$TIER" in
  UNDER)
    echo -e "  pressure:            ${GREEN}ADMITTED${NC} (under soft threshold)"
    ;;
  SOFT)
    echo -e "  pressure:            ${YELLOW}PARTIAL${NC} (over soft threshold — soft prune indicated)"
    ;;
  HARD)
    if [ "$FREE_MB" -lt "$LOW_FREE_MB" ]; then
      echo -e "  pressure:            ${RED}BLOCKED${NC} (free space < ${LOW_FREE_MB} MB — hard escalation indicated)"
    else
      echo -e "  pressure:            ${RED}BLOCKED${NC} (over hard cap — hard escalation indicated)"
    fi
    ;;
esac

# ----------------------------------------------------------------------------
# Plan / apply
# ----------------------------------------------------------------------------
echo -e "\n${BLUE}[3/3] Reclamation plan...${NC}"

# Build the ordered prune list (paths that exist).
PRUNE_LIST=()
[ -d "$DOC_DIR" ] && PRUNE_LIST+=("$DOC_DIR")
for d in "${INCR_DIRS[@]:-}"; do [ -n "${d:-}" ] && PRUNE_LIST+=("$d"); done
for p in "${STALE_PATHS[@]:-}"; do [ -n "${p:-}" ] && PRUNE_LIST+=("$p"); done

EXIT_CODE=0

if [ "$TIER" = "UNDER" ]; then
  echo -e "  no action: $(fmt_mb "$TARGET_TOTAL_MB") is within the ${SOFT_MB} MB budget."
  echo -e "\nBudget status:        ${GREEN}ADMITTED${NC}"
  exit 0
fi

# SOFT (and HARD) both start with the soft prune set.
if [ "${#PRUNE_LIST[@]}" -eq 0 ]; then
  echo -e "  ${YELLOW}OPEN${NC}: over threshold but no soft-prune candidates present."
  echo -e "       (doc/incremental/stale all empty — only compiled artifacts remain)"
else
  echo -e "  soft prune would reclaim ~$(fmt_mb "$SOFT_RECLAIM_MB") across:"
  for p in "${PRUNE_LIST[@]}"; do
    echo -e "    - ${p#$PROJECT_ROOT/}"
  done

  if [ "$APPLY" -eq 1 ]; then
    echo -e "\n  ${YELLOW}--apply set: removing soft-prune paths...${NC}"
    for p in "${PRUNE_LIST[@]}"; do
      rm -rf -- "$p"
      echo -e "    pruned ${p#$PROJECT_ROOT/}"
    done
    echo -e "  soft prune carried out:   ${GREEN}ADMITTED${NC}"
  else
    echo -e "\n  ${BLUE}dry-run: no paths removed. Re-run with --apply to prune.${NC}"
  fi
fi

# HARD escalation: cargo clean, gated on --apply AND past the hard cap.
if [ "$TIER" = "HARD" ]; then
  echo -e "\n  ${RED}HARD escalation candidate: full 'cargo clean'${NC}"
  echo -e "    would remove the entire target dir (~$(fmt_mb "$TARGET_TOTAL_MB"))."
  if [ "$APPLY" -eq 1 ]; then
    echo -e "  ${YELLOW}--apply set + hard pressure: invoking cargo clean...${NC}"
    if command -v cargo >/dev/null 2>&1; then
      ( cd "$PROJECT_ROOT" && cargo clean )
      echo -e "  cargo clean carried out:  ${GREEN}ADMITTED${NC}"
    else
      echo -e "  cargo not on PATH:        ${YELLOW}UNKNOWN${NC} (escalation not carried out)"
      EXIT_CODE=2
    fi
  else
    echo -e "  ${BLUE}dry-run: cargo clean NOT invoked. Re-run with --apply to escalate.${NC}"
    # Signal hard pressure to the caller even in dry-run.
    EXIT_CODE=2
  fi
fi

# ----------------------------------------------------------------------------
# Summary
# ----------------------------------------------------------------------------
echo -e "\n${MAGENTA}========================================${NC}"
case "$TIER" in
  SOFT)
    if [ "$APPLY" -eq 1 ]; then
      echo -e "Budget status:        ${YELLOW}PARTIAL${NC} (soft prune carried out; recheck size)"
    else
      echo -e "Budget status:        ${YELLOW}PARTIAL${NC} (soft prune planned, not applied)"
    fi
    ;;
  HARD)
    echo -e "Budget status:        ${RED}BLOCKED${NC} (hard cap pressure; escalation gated on --apply)"
    ;;
esac
echo -e "${MAGENTA}========================================${NC}"

exit "$EXIT_CODE"
