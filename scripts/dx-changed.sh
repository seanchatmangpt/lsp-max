#!/usr/bin/env bash
#
# scripts/dx-changed.sh
#
# Fast iteration loop: format + lint ONLY the workspace crates touched since a
# base ref, instead of the whole workspace. Changed files are mapped to their
# owning crate via `cargo metadata` (each crate's manifest_path defines its
# directory; a changed file under that directory belongs to that crate).
#
# Then runs, scoped per owning crate:
#   cargo fmt -p <crate> -- --check    (read-only check; does not rewrite files)
#   cargo clippy -p <crate> --all-targets --all-features -- -D warnings
#
# Both run on the pinned toolchain from rust-toolchain.toml so the scoped result
# tracks the same compiler CI uses.
#
# Usage:  scripts/dx-changed.sh [BASE_REF]
#         BASE_REF defaults to origin/master.
#
# Exit codes:
#   0 = changed crates are ADMITTED for fmt + clippy (or no crates changed)
#   1 = at least one scoped gate BLOCKED
#   2 = preconditions UNKNOWN (no toolchain / no metadata tool / not a repo)

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

BASE_REF="${1:-origin/master}"

# ----------------------------------------------------------------------------
# Preconditions.
# ----------------------------------------------------------------------------
if ! command -v jq >/dev/null 2>&1; then
  echo -e "${YELLOW}jq not found; crate mapping requires jq.${NC}"
  echo -e "Status: UNKNOWN (cannot parse cargo metadata)"
  exit 2
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo -e "${YELLOW}Not inside a git work tree.${NC}"
  echo -e "Status: UNKNOWN (cannot diff against a base ref)"
  exit 2
fi

TOOLCHAIN_FILE="$PROJECT_ROOT/rust-toolchain.toml"
PINNED_CHANNEL=""
if [ -f "$TOOLCHAIN_FILE" ]; then
  PINNED_CHANNEL="$(
    grep -E '^[[:space:]]*channel[[:space:]]*=' "$TOOLCHAIN_FILE" \
      | head -n 1 \
      | sed -E 's/.*=[[:space:]]*"([^"]+)".*/\1/'
  )"
fi

CARGO=(cargo)
if [ -n "$PINNED_CHANNEL" ]; then
  if rustup toolchain list 2>/dev/null | grep -q "$PINNED_CHANNEL"; then
    CARGO=(cargo "+${PINNED_CHANNEL}")
  else
    echo -e "${YELLOW}Pinned toolchain '${PINNED_CHANNEL}' absent; using default cargo.${NC}"
    echo -e "${YELLOW}Scoped result may diverge from CI. Status of toolchain parity: UNKNOWN${NC}"
  fi
fi

echo -e "${MAGENTA}============================================================${NC}"
echo -e "${BLUE}dx-changed: scoped fmt + clippy vs ${BASE_REF}${NC}"
echo -e "${MAGENTA}============================================================${NC}"

# ----------------------------------------------------------------------------
# Resolve a diff base. If BASE_REF is unknown to git (e.g. no remote fetched),
# fall back to the merge-base with HEAD when possible; otherwise report UNKNOWN
# rather than silently diffing nothing.
# ----------------------------------------------------------------------------
DIFF_BASE="$BASE_REF"
if ! git rev-parse --verify --quiet "$BASE_REF" >/dev/null 2>&1; then
  echo -e "${YELLOW}Base ref '${BASE_REF}' not found locally.${NC}"
  echo -e "Status: UNKNOWN (cannot determine changed set without a valid base)"
  exit 2
fi
if MB="$(git merge-base "$BASE_REF" HEAD 2>/dev/null)"; then
  DIFF_BASE="$MB"
fi

# Changed paths = committed diff vs base, plus working-tree + staged changes,
# so an in-progress edit is linted before it is committed.
mapfile -t CHANGED_FILES < <(
  {
    git diff --name-only "$DIFF_BASE"...HEAD
    git diff --name-only HEAD
    git diff --name-only --cached
  } 2>/dev/null | sort -u
)

if [ "${#CHANGED_FILES[@]}" -eq 0 ]; then
  echo -e "${BLUE}No changed files vs ${BASE_REF}.${NC}"
  echo -e "Status: ADMITTED (empty changed set; nothing to lint)"
  exit 0
fi

# ----------------------------------------------------------------------------
# Build a (crate-dir -> crate-name) table for workspace members from metadata.
# ----------------------------------------------------------------------------
METADATA="$("${CARGO[@]}" metadata --no-deps --format-version 1 2>/dev/null)" || {
  echo -e "${RED}cargo metadata failed.${NC}"
  echo -e "Status: UNKNOWN (cannot enumerate workspace crates)"
  exit 2
}

# Emit "<absolute-crate-dir>\t<crate-name>" per workspace package.
declare -a CRATE_DIRS=()
declare -a CRATE_NAMES=()
while IFS=$'\t' read -r cdir cname; do
  [ -n "$cdir" ] || continue
  CRATE_DIRS+=("$cdir")
  CRATE_NAMES+=("$cname")
done < <(
  echo "$METADATA" \
    | jq -r '.packages[] | [(.manifest_path | rtrimstr("/Cargo.toml")), .name] | @tsv'
)

# ----------------------------------------------------------------------------
# Map each changed file to the longest matching crate directory (most specific
# wins — handles nested members like crates/lsp-max-adapters/lsp-max-ast).
# ----------------------------------------------------------------------------
declare -A SELECTED=()
for f in "${CHANGED_FILES[@]}"; do
  [ -n "$f" ] || continue
  abs="$PROJECT_ROOT/$f"
  best_len=-1
  best_name=""
  for i in "${!CRATE_DIRS[@]}"; do
    cdir="${CRATE_DIRS[$i]}"
    case "$abs" in
      "$cdir"/*|"$cdir")
        len="${#cdir}"
        if [ "$len" -gt "$best_len" ]; then
          best_len="$len"
          best_name="${CRATE_NAMES[$i]}"
        fi
        ;;
    esac
  done
  if [ -n "$best_name" ]; then
    SELECTED["$best_name"]=1
  fi
done

if [ "${#SELECTED[@]}" -eq 0 ]; then
  echo -e "${BLUE}Changed files map to no workspace crate (docs/config only):${NC}"
  printf '    %s\n' "${CHANGED_FILES[@]}"
  echo -e "Status: ADMITTED (no crate sources changed; nothing to lint)"
  exit 0
fi

mapfile -t CRATES < <(printf '%s\n' "${!SELECTED[@]}" | sort)

echo -e "${BLUE}Changed crates (${#CRATES[@]}):${NC} ${CRATES[*]}"
echo ""

# ----------------------------------------------------------------------------
# Scoped fmt + clippy, per changed crate. -p flags accumulate.
# ----------------------------------------------------------------------------
PKG_ARGS=()
for c in "${CRATES[@]}"; do
  PKG_ARGS+=(-p "$c")
done

FAILED=0
FMT_STATUS="UNKNOWN"
CLIPPY_STATUS="UNKNOWN"

echo -e "${BLUE}► [fmt] cargo fmt ${PKG_ARGS[*]} -- --check${NC}"
if "${CARGO[@]}" fmt "${PKG_ARGS[@]}" -- --check; then
  FMT_STATUS="ADMITTED"
else
  FMT_STATUS="BLOCKED"
  FAILED=1
fi
echo ""

echo -e "${BLUE}► [clippy] cargo clippy ${PKG_ARGS[*]} --all-targets --all-features -- -D warnings${NC}"
if "${CARGO[@]}" clippy "${PKG_ARGS[@]}" --all-targets --all-features -- -D warnings; then
  CLIPPY_STATUS="ADMITTED"
else
  CLIPPY_STATUS="BLOCKED"
  FAILED=1
fi
echo ""

echo -e "${MAGENTA}============================================================${NC}"
printf "%-10s %s\n" "GATE" "STATUS"
for pair in "fmt:$FMT_STATUS" "clippy:$CLIPPY_STATUS"; do
  name="${pair%%:*}"; st="${pair##*:}"
  case "$st" in
    ADMITTED) col="${GREEN}" ;;
    BLOCKED)  col="${RED}" ;;
    *)        col="${YELLOW}" ;;
  esac
  printf "%-10s ${col}%s${NC}\n" "$name" "$st"
done
echo ""

if [ "$FAILED" -eq 0 ]; then
  echo -e "${GREEN}dx-changed verdict: ADMITTED (scoped fmt + clippy passed for changed crates)${NC}"
  exit 0
else
  echo -e "${RED}dx-changed verdict: BLOCKED (scoped fmt or clippy failed)${NC}"
  exit 1
fi
