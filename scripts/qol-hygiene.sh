#!/bin/bash
#
# scripts/qol-hygiene.sh
#
# AutoQoL: workspace hygiene audit.
#
# Surfaces two classes of repository smell and prints the exact, bounded
# remediation for each. READ-ONLY by default — it reports and never mutates
# the git index or working tree. Pass `--apply` to carry out ONLY the safe
# remediation (`git rm --cached <path>`); it will never delete working files
# and never touches file contents.
#
# Audited smells:
#
#   (A) Tracked-but-ignored runtime artifacts
#       Files that match a .gitignore rule yet still appear in `git ls-files`.
#       These are usually build/runtime droppings committed before the ignore
#       rule existed. Detection uses `git check-ignore`, which honors negated
#       (`!`) re-include rules — so intentionally re-tracked files (e.g. the
#       committed generated/ snapshots that .gitignore deliberately un-ignores)
#       are NOT flagged. Remediation: `git rm --cached <path>`.
#
#   (B) Test-written tree residue
#       Working-tree paths matching known test-output shapes (target_e2e/,
#       lsif_dump.json, refund_receipt.txt, scratch/, *.rs.bk) that tests are
#       known to write into the source tree. Reported so they can be cleaned
#       or redirected; never auto-deleted.
#
# Output uses bounded statuses only (ADMITTED / CANDIDATE / BLOCKED /
# PARTIAL / OPEN / UNKNOWN). No victory language.
#
# Exit codes:
#   0 = no hygiene smells found (audit ADMITTED)
#   1 = at least one smell found (audit reports OPEN items to remediate)
#
# Usage:
#   scripts/qol-hygiene.sh            # read-only audit (default)
#   scripts/qol-hygiene.sh --apply    # also run `git rm --cached` on (A) hits

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
while [ $# -gt 0 ]; do
  case "$1" in
    --apply) APPLY=1; shift ;;
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

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo -e "${RED}BLOCKED: not inside a git work tree.${NC}" >&2
  exit 1
fi

echo -e "${MAGENTA}========================================${NC}"
echo -e "${CYAN} AutoQoL: Workspace Hygiene Audit${NC}"
echo -e "${MAGENTA}========================================${NC}"
if [ "$APPLY" -eq 1 ]; then
  echo -e "${YELLOW}mode: --apply (git rm --cached authorized for tracked-but-ignored hits)${NC}"
else
  echo -e "${BLUE}mode: read-only (report only; pass --apply to un-track ignored hits)${NC}"
fi

SMELLS=0

# ============================================================================
# (A) Tracked-but-ignored runtime artifacts
# ============================================================================
echo -e "\n${BLUE}[A] Tracked files that match a .gitignore rule...${NC}"

# `git check-ignore --stdin` returns (on stdout) only the paths that an ignore
# rule matches, AFTER applying negated re-include rules. Feeding it the full
# tracked set yields exactly the tracked-but-ignored intersection.
TRACKED_IGNORED="$(
  git ls-files -z \
    | git check-ignore --stdin -z 2>/dev/null \
    | tr '\0' '\n' || true
)"

if [ -z "$TRACKED_IGNORED" ]; then
  echo -e "  ${GREEN}ADMITTED${NC}: no tracked file matches an active ignore rule."
else
  COUNT_A="$(printf '%s\n' "$TRACKED_IGNORED" | grep -c . || true)"
  echo -e "  ${RED}OPEN${NC}: ${COUNT_A} tracked path(s) match a .gitignore rule (likely runtime residue):"
  while IFS= read -r path; do
    [ -n "$path" ] || continue
    echo -e "    ${YELLOW}$path${NC}"
    echo -e "      remediation: ${CYAN}git rm --cached -- \"$path\"${NC}"
    SMELLS=$((SMELLS + 1))
  done <<< "$TRACKED_IGNORED"

  if [ "$APPLY" -eq 1 ]; then
    echo -e "\n  ${YELLOW}--apply set: un-tracking (index only; working files preserved)...${NC}"
    while IFS= read -r path; do
      [ -n "$path" ] || continue
      # --cached: removes from index only. Never deletes the working file.
      git rm --cached --quiet -- "$path" \
        && echo -e "    un-tracked ${path}   ${GREEN}ADMITTED${NC}" \
        || echo -e "    ${RED}BLOCKED${NC} un-tracking ${path}"
    done <<< "$TRACKED_IGNORED"
    echo -e "  ${BLUE}note: review with 'git status' and commit the index change yourself.${NC}"
  else
    echo -e "\n  ${BLUE}read-only: index unchanged. Re-run with --apply to un-track these.${NC}"
  fi
fi

# ============================================================================
# (B) Test-written tree residue
# ============================================================================
echo -e "\n${BLUE}[B] Test-output residue present in the working tree...${NC}"

# Shapes that test code in this workspace is known to write into the tree.
# (Mirrors .gitignore runtime-artifact entries: target_e2e/, lsif_dump.json,
#  refund_receipt.txt, scratch/, *.rs.bk.)
RESIDUE_GLOBS=(
  "*/target_e2e"
  "*/lsif_dump.json"
  "lsif_dump.json"
  "*/refund_receipt.txt"
  "refund_receipt.txt"
  "*.rs.bk"
  "scratch"
)

RESIDUE_HITS=()
for g in "${RESIDUE_GLOBS[@]}"; do
  while IFS= read -r hit; do
    [ -n "$hit" ] || continue
    RESIDUE_HITS+=("$hit")
  done < <(find "$PROJECT_ROOT" \
              -path "$PROJECT_ROOT/target" -prune -o \
              -path "$PROJECT_ROOT/.git" -prune -o \
              -name "$(basename "$g")" -print 2>/dev/null || true)
done

# De-duplicate.
if [ "${#RESIDUE_HITS[@]}" -gt 0 ]; then
  mapfile -t RESIDUE_HITS < <(printf '%s\n' "${RESIDUE_HITS[@]}" | sort -u)
fi

if [ "${#RESIDUE_HITS[@]}" -eq 0 ]; then
  echo -e "  ${GREEN}ADMITTED${NC}: no known test-output residue in the tree."
else
  echo -e "  ${YELLOW}CANDIDATE${NC}: ${#RESIDUE_HITS[@]} path(s) look like test-written residue:"
  for hit in "${RESIDUE_HITS[@]}"; do
    rel="${hit#$PROJECT_ROOT/}"
    # Is it tracked? If so, the remediation is git rm --cached; else it is just
    # a working-tree dropping the dev can remove manually.
    if git ls-files --error-unmatch -- "$rel" >/dev/null 2>&1; then
      echo -e "    ${YELLOW}$rel${NC}  (tracked)"
      echo -e "      remediation: ${CYAN}git rm --cached -- \"$rel\"${NC}  then add to .gitignore if absent"
      SMELLS=$((SMELLS + 1))
    else
      echo -e "    ${YELLOW}$rel${NC}  (untracked working-tree residue)"
      echo -e "      remediation: remove manually or have the test write outside the source tree"
    fi
  done
  echo -e "\n  ${BLUE}read-only: this audit never deletes working files.${NC}"
fi

# ============================================================================
# Summary
# ============================================================================
echo -e "\n${MAGENTA}========================================${NC}"
if [ "$SMELLS" -eq 0 ]; then
  echo -e "Hygiene status:       ${GREEN}ADMITTED${NC} (no tracked-but-ignored or tracked-residue smells)"
  echo -e "${MAGENTA}========================================${NC}"
  exit 0
else
  echo -e "Hygiene status:       ${YELLOW}OPEN${NC} (${SMELLS} remediable smell(s) reported above)"
  echo -e "${MAGENTA}========================================${NC}"
  exit 1
fi
