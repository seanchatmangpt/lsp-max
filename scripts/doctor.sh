#!/bin/bash
#
# scripts/doctor.sh
#
# One-shot, READ-ONLY environment & workspace health diagnostic for lsp-max.
# This is the first thing anyone should run: it catches the precondition
# failures this workspace actually hits before a build/test is even attempted.
#
# It NEVER mutates tracked files, manifests, or sibling repos. It only reads
# the filesystem, git index, and the ANDON gate file, then reports a bounded
# status per axis.
#
# Axes:
#   1. sibling checkouts present (../lsp-types-max, ../wasm4pm-compat, ../wasm4pm)
#   2. each sibling's actual version vs. the path-dep `version=` this workspace
#      requires (a requirement the sibling does not satisfy => BLOCKED)
#   3. rust-toolchain.toml channel vs. the toolchain pinned in CI workflows
#   4. free disk headroom on the workspace volume
#   5. committed merge-conflict markers in tracked source
#   6. path-dependency depth sanity (each `path=` resolves to a real Cargo.toml)
#   7. tracked-but-gitignored runtime artifacts
#   8. ANDON gate state via `lsp-max-cli gate check` (absent binary => UNKNOWN)
#
# Bounded statuses only (never collapse UNKNOWN into ADMITTED/REFUSED):
#   ADMITTED  — axis clear
#   PARTIAL   — axis degraded but not hard-blocking
#   BLOCKED   — axis is a hard precondition failure
#   UNKNOWN   — axis could not be determined (missing tool/input)
#
# Overall verdict is ADMITTED only if every axis is ADMITTED; otherwise it is
# the most severe of BLOCKED > PARTIAL > UNKNOWN observed.
#
# Exit codes:
#   0 = overall ADMITTED
#   1 = overall BLOCKED
#   2 = overall PARTIAL
#   3 = overall UNKNOWN
#
# Flags:
#   --json   emit only the machine-readable block (no human table, no color)

set -euo pipefail

# ----------------------------------------------------------------------------
# Presentation
# ----------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

JSON_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --json) JSON_ONLY=1 ;;
    *) ;;
  esac
done

if [ "$JSON_ONLY" -eq 1 ]; then
  RED=''; GREEN=''; YELLOW=''; BLUE=''; MAGENTA=''; CYAN=''; NC=''
fi

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"
PARENT_DIR="$(cd "$PROJECT_ROOT/.." && pwd)"

# Minimum free headroom (GiB) before the volume is flagged.
DISK_PARTIAL_GIB="${LSP_MAX_DOCTOR_DISK_PARTIAL_GIB:-5}"
DISK_BLOCKED_GIB="${LSP_MAX_DOCTOR_DISK_BLOCKED_GIB:-1}"

# Axis records: name|status|detail  (detail carries the fix hint)
AXES=()

record() {
  # record <name> <status> <detail>
  AXES+=("$1|$2|$3")
}

emit_human_row() {
  local name="$1" status="$2" detail="$3" color
  case "$status" in
    ADMITTED) color="$GREEN" ;;
    PARTIAL)  color="$YELLOW" ;;
    BLOCKED)  color="$RED" ;;
    UNKNOWN)  color="$CYAN" ;;
    *)        color="$NC" ;;
  esac
  printf "  %-26s ${color}%-9s${NC} %s\n" "$name" "$status" "$detail"
}

# CalVer / SemVer caret comparison helper.
# semver_satisfies <actual> <required>  -> "yes" | "no" | "unknown"
# Treats `required` as a caret floor: actual must be >= required and share the
# same nonzero leading component (major), matching Cargo's default `version=`.
semver_satisfies() {
  local actual="$1" required="$2"
  if ! [[ "$actual" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]] \
     || ! [[ "$required" =~ ^[0-9]+(\.[0-9]+){0,2}$ ]]; then
    echo "unknown"
    return
  fi
  local a_major a_minor a_patch r_major r_minor r_patch
  a_major="${actual%%.*}"; a_minor="$(echo "$actual" | cut -s -d. -f2)"; a_patch="$(echo "$actual" | cut -s -d. -f3)"
  r_major="${required%%.*}"; r_minor="$(echo "$required" | cut -s -d. -f2)"; r_patch="$(echo "$required" | cut -s -d. -f3)"
  a_minor="${a_minor:-0}"; a_patch="${a_patch:-0}"
  r_minor="${r_minor:-0}"; r_patch="${r_patch:-0}"

  # Caret pins the leading nonzero component. For 26.x.y that is the major (26).
  if [ "$a_major" != "$r_major" ]; then
    echo "no"; return
  fi
  if [ "$a_minor" -gt "$r_minor" ]; then echo "yes"; return; fi
  if [ "$a_minor" -lt "$r_minor" ]; then echo "no"; return; fi
  if [ "$a_patch" -ge "$r_patch" ]; then echo "yes"; else echo "no"; fi
}

# Read a sibling crate's declared version (first top-level `version = "..."`).
sibling_version() {
  local manifest="$1"
  [ -f "$manifest" ] || { echo ""; return; }
  rg -N -m1 '^\s*version\s*=\s*"([^"]+)"' "$manifest" -o -r '$1' 2>/dev/null || echo ""
}

# ----------------------------------------------------------------------------
# Axis 1 + 2: sibling checkouts present, and version vs. required path-dep floor
# ----------------------------------------------------------------------------
# required-floor table: the `version=` each path dep declares in this workspace.
# These mirror Cargo.toml / crates/lsp-max-cli/Cargo.toml path-dep version= keys.
declare -A SIBLING_DIR=(
  ["lsp-types-max"]="$PARENT_DIR/lsp-types-max"
  ["wasm4pm-compat"]="$PARENT_DIR/wasm4pm-compat"
  ["wasm4pm"]="$PARENT_DIR/wasm4pm"
)
declare -A SIBLING_MANIFEST=(
  ["lsp-types-max"]="$PARENT_DIR/lsp-types-max/Cargo.toml"
  ["wasm4pm-compat"]="$PARENT_DIR/wasm4pm-compat/Cargo.toml"
  ["wasm4pm"]="$PARENT_DIR/wasm4pm/wasm4pm/Cargo.toml"
)

# Discover the required floor straight from this workspace's manifests so the
# doctor never drifts from the source of truth.
discover_required() {
  # discover_required <crate-name> -> version floor string (may be empty)
  local crate="$1" v=""
  v="$(rg -N -o -r '$1' \
        "$crate\s*=\s*\{[^}]*version\s*=\s*\"([^\"]+)\"" \
        "$PROJECT_ROOT/Cargo.toml" "$PROJECT_ROOT/crates/lsp-max-cli/Cargo.toml" \
        2>/dev/null | head -n1 || true)"
  echo "$v"
}

SIBLINGS_PRESENT="ADMITTED"
SIBLINGS_PRESENT_DETAIL="all three sibling checkouts present"
MISSING_SIBLINGS=""
for name in lsp-types-max wasm4pm-compat wasm4pm; do
  if [ ! -d "${SIBLING_DIR[$name]}" ]; then
    MISSING_SIBLINGS="$MISSING_SIBLINGS $name"
  fi
done
if [ -n "$MISSING_SIBLINGS" ]; then
  SIBLINGS_PRESENT="BLOCKED"
  SIBLINGS_PRESENT_DETAIL="missing:${MISSING_SIBLINGS} — clone into ${PARENT_DIR}/ (path deps + [patch.crates-io] require them on disk)"
fi
record "siblings.present" "$SIBLINGS_PRESENT" "$SIBLINGS_PRESENT_DETAIL"

VER_STATUS="ADMITTED"
VER_DETAIL="every sibling satisfies its required version floor"
VER_NOTES=""
for name in lsp-types-max wasm4pm-compat wasm4pm; do
  manifest="${SIBLING_MANIFEST[$name]}"
  actual="$(sibling_version "$manifest")"
  required="$(discover_required "$name")"
  if [ -z "$actual" ]; then
    [ "$VER_STATUS" = "ADMITTED" ] && VER_STATUS="UNKNOWN"
    VER_NOTES="$VER_NOTES ${name}=<no-manifest-version>"
    continue
  fi
  if [ -z "$required" ]; then
    # No version= floor declared for this dep (e.g. patched-only). Cannot mismatch.
    VER_NOTES="$VER_NOTES ${name}:${actual}(no-floor)"
    continue
  fi
  ok="$(semver_satisfies "$actual" "$required")"
  case "$ok" in
    yes) VER_NOTES="$VER_NOTES ${name}:${actual}>=^${required}" ;;
    no)
      VER_STATUS="BLOCKED"
      VER_NOTES="$VER_NOTES ${name}:${actual}!<^${required}"
      ;;
    unknown)
      [ "$VER_STATUS" = "ADMITTED" ] && VER_STATUS="UNKNOWN"
      VER_NOTES="$VER_NOTES ${name}:${actual}?${required}"
      ;;
  esac
done
if [ "$VER_STATUS" = "BLOCKED" ]; then
  VER_DETAIL="sibling version below required floor —${VER_NOTES} — bump sibling or align path-dep version="
elif [ "$VER_STATUS" = "UNKNOWN" ]; then
  VER_DETAIL="could not resolve a version —${VER_NOTES}"
else
  VER_DETAIL="floors satisfied —${VER_NOTES}"
fi
record "siblings.version" "$VER_STATUS" "$VER_DETAIL"

# ----------------------------------------------------------------------------
# Axis 3: rust-toolchain.toml channel vs. CI-pinned toolchain
# ----------------------------------------------------------------------------
TC_STATUS="ADMITTED"
TC_DETAIL=""
PINNED=""
if [ -f "$PROJECT_ROOT/rust-toolchain.toml" ]; then
  PINNED="$(rg -N -o -r '$1' '^\s*channel\s*=\s*"([^"]+)"' "$PROJECT_ROOT/rust-toolchain.toml" 2>/dev/null | head -n1 || true)"
fi
if [ -z "$PINNED" ]; then
  TC_STATUS="UNKNOWN"
  TC_DETAIL="rust-toolchain.toml channel not found"
else
  CI_TOOLCHAINS="$(rg -N -o -r '$1' 'toolchain:\s*([0-9A-Za-z._-]+)' "$PROJECT_ROOT/.github/workflows/" 2>/dev/null | sort -u || true)"
  if [ -z "$CI_TOOLCHAINS" ]; then
    TC_STATUS="UNKNOWN"
    TC_DETAIL="pinned=${PINNED}; no toolchain found in .github/workflows/"
  else
    DRIFT=""
    while IFS= read -r ci_tc; do
      [ -z "$ci_tc" ] && continue
      if [ "$ci_tc" != "$PINNED" ]; then
        DRIFT="$DRIFT ${ci_tc}"
      fi
    done <<< "$CI_TOOLCHAINS"
    if [ -n "$DRIFT" ]; then
      TC_STATUS="BLOCKED"
      TC_DETAIL="drift — rust-toolchain.toml=${PINNED} vs CI={${DRIFT} }; align both to the same channel"
    else
      TC_DETAIL="pinned=${PINNED} matches CI workflows"
    fi
  fi
fi
record "toolchain.pin" "$TC_STATUS" "$TC_DETAIL"

# ----------------------------------------------------------------------------
# Axis 4: free disk headroom on the workspace volume
# ----------------------------------------------------------------------------
DISK_STATUS="UNKNOWN"
DISK_DETAIL="df unavailable"
if command -v df >/dev/null 2>&1; then
  AVAIL_KB="$(df -Pk "$PROJECT_ROOT" 2>/dev/null | awk 'NR==2 {print $4}' || true)"
  if [[ "$AVAIL_KB" =~ ^[0-9]+$ ]]; then
    AVAIL_GIB=$(( AVAIL_KB / 1024 / 1024 ))
    if [ "$AVAIL_GIB" -lt "$DISK_BLOCKED_GIB" ]; then
      DISK_STATUS="BLOCKED"
      DISK_DETAIL="${AVAIL_GIB}GiB free (< ${DISK_BLOCKED_GIB}GiB) — a build will hit 'No space left on device'; free space before building"
    elif [ "$AVAIL_GIB" -lt "$DISK_PARTIAL_GIB" ]; then
      DISK_STATUS="PARTIAL"
      DISK_DETAIL="${AVAIL_GIB}GiB free (< ${DISK_PARTIAL_GIB}GiB) — headroom is thin for a full workspace build"
    else
      DISK_STATUS="ADMITTED"
      DISK_DETAIL="${AVAIL_GIB}GiB free on workspace volume"
    fi
  fi
fi
record "disk.headroom" "$DISK_STATUS" "$DISK_DETAIL"

# ----------------------------------------------------------------------------
# Axis 5: committed merge-conflict markers in tracked source
# ----------------------------------------------------------------------------
CONFLICT_STATUS="ADMITTED"
CONFLICT_DETAIL="no conflict markers in tracked source"
# Match 7+ run of the conflict sigils at line start. Exclude this script itself
# (it documents the markers) and the doctor noun source.
CONFLICT_HITS="$(rg -n -l '^(<{7}|={7}|>{7})' \
  --glob '!target/**' --glob '!.git/**' \
  --glob '!scripts/doctor.sh' \
  --glob '!**/doctor.rs' \
  . 2>/dev/null || true)"
if [ -n "$CONFLICT_HITS" ]; then
  # Keep only files git is actually tracking.
  TRACKED_CONFLICTS=""
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    rel="${f#./}"
    if git ls-files --error-unmatch -- "$rel" >/dev/null 2>&1; then
      TRACKED_CONFLICTS="$TRACKED_CONFLICTS $rel"
    fi
  done <<< "$CONFLICT_HITS"
  if [ -n "$TRACKED_CONFLICTS" ]; then
    CONFLICT_STATUS="BLOCKED"
    CONFLICT_DETAIL="conflict markers in:${TRACKED_CONFLICTS} — resolve the merge before building"
  fi
fi
record "git.conflicts" "$CONFLICT_STATUS" "$CONFLICT_DETAIL"

# ----------------------------------------------------------------------------
# Axis 6: path-dependency depth sanity
# ----------------------------------------------------------------------------
PATHDEP_STATUS="ADMITTED"
PATHDEP_DETAIL="every workspace path= resolves to a real Cargo.toml"
BROKEN_PATHS=""
CHECKED_PATHS=0
# Iterate every tracked Cargo.toml; for each `path = "..."` resolve relative to
# that manifest's directory and confirm a Cargo.toml lives there. This catches
# the ../../ vs ../../../ depth bug that silently detaches a dependency.
while IFS= read -r manifest; do
  [ -z "$manifest" ] && continue
  mdir="$(dirname "$manifest")"
  while IFS= read -r relpath; do
    [ -z "$relpath" ] && continue
    CHECKED_PATHS=$(( CHECKED_PATHS + 1 ))
    target="$mdir/$relpath"
    if [ ! -f "$target/Cargo.toml" ] && [ ! -f "$target" ]; then
      BROKEN_PATHS="$BROKEN_PATHS ${manifest#./}=>${relpath}"
    fi
  done < <(rg -N -o -r '$1' 'path\s*=\s*"([^"]+)"' "$manifest" 2>/dev/null || true)
done < <(git ls-files -- '**/Cargo.toml' 'Cargo.toml' 2>/dev/null || true)

if [ -n "$BROKEN_PATHS" ]; then
  PATHDEP_STATUS="BLOCKED"
  PATHDEP_DETAIL="unresolved path deps:${BROKEN_PATHS} — fix the ../ depth so it points at a Cargo.toml"
else
  PATHDEP_DETAIL="${CHECKED_PATHS} path dep(s) resolve to a real Cargo.toml"
fi
record "manifests.pathdeps" "$PATHDEP_STATUS" "$PATHDEP_DETAIL"

# ----------------------------------------------------------------------------
# Axis 7: tracked-but-gitignored runtime artifacts
# ----------------------------------------------------------------------------
IGNORED_STATUS="ADMITTED"
IGNORED_DETAIL="no tracked file matches .gitignore"
TRACKED_IGNORED=""
if [ -f "$PROJECT_ROOT/.gitignore" ]; then
  # git check-ignore against the tracked set surfaces files that are both
  # committed AND match an ignore rule — a runtime artifact that leaked in.
  while IFS= read -r leaked; do
    [ -z "$leaked" ] && continue
    TRACKED_IGNORED="$TRACKED_IGNORED $leaked"
  done < <(git ls-files 2>/dev/null | git check-ignore --stdin 2>/dev/null || true)
fi
if [ -n "$TRACKED_IGNORED" ]; then
  IGNORED_STATUS="PARTIAL"
  COUNT="$(echo "$TRACKED_IGNORED" | wc -w | tr -d ' ')"
  FIRST="$(echo "$TRACKED_IGNORED" | tr ' ' '\n' | grep -v '^$' | head -n5 | tr '\n' ' ')"
  IGNORED_DETAIL="${COUNT} tracked file(s) match .gitignore (e.g.${FIRST}) — git rm --cached the leaked runtime artifacts"
fi
record "git.ignored_tracked" "$IGNORED_STATUS" "$IGNORED_DETAIL"

# ----------------------------------------------------------------------------
# Axis 8: ANDON gate state (best-effort; absent binary => UNKNOWN, never fail)
# ----------------------------------------------------------------------------
GATE_STATUS="UNKNOWN"
GATE_DETAIL="lsp-max-cli not on PATH — gate state not observed"
if command -v lsp-max-cli >/dev/null 2>&1; then
  if lsp-max-cli gate check >/dev/null 2>&1; then
    GATE_STATUS="ADMITTED"
    GATE_DETAIL="ANDON gate is clear (lsp-max-cli gate check exit 0)"
  else
    GATE_STATUS="BLOCKED"
    GATE_DETAIL="ANDON gate is set — resolve active WASM4PM-* / GGEN-* diagnostics before shell actions"
  fi
fi
record "andon.gate" "$GATE_STATUS" "$GATE_DETAIL"

# ----------------------------------------------------------------------------
# Roll up overall verdict: ADMITTED only if all axes ADMITTED; else most severe
# of BLOCKED > PARTIAL > UNKNOWN. UNKNOWN never collapses into a polarity.
# ----------------------------------------------------------------------------
HAS_BLOCKED=0; HAS_PARTIAL=0; HAS_UNKNOWN=0
for rec in "${AXES[@]}"; do
  st="$(echo "$rec" | cut -d'|' -f2)"
  case "$st" in
    BLOCKED) HAS_BLOCKED=1 ;;
    PARTIAL) HAS_PARTIAL=1 ;;
    UNKNOWN) HAS_UNKNOWN=1 ;;
  esac
done

if [ "$HAS_BLOCKED" -eq 1 ]; then
  OVERALL="BLOCKED"; EXIT_CODE=1
elif [ "$HAS_PARTIAL" -eq 1 ]; then
  OVERALL="PARTIAL"; EXIT_CODE=2
elif [ "$HAS_UNKNOWN" -eq 1 ]; then
  OVERALL="UNKNOWN"; EXIT_CODE=3
else
  OVERALL="ADMITTED"; EXIT_CODE=0
fi

# ----------------------------------------------------------------------------
# Human-readable table
# ----------------------------------------------------------------------------
if [ "$JSON_ONLY" -eq 0 ]; then
  echo -e "${MAGENTA}============================================================${NC}"
  echo -e "${CYAN} lsp-max doctor — read-only environment & workspace check ${NC}"
  echo -e "${MAGENTA}============================================================${NC}"
  echo -e "  root: ${PROJECT_ROOT}"
  echo ""
  for rec in "${AXES[@]}"; do
    name="$(echo "$rec" | cut -d'|' -f1)"
    status="$(echo "$rec" | cut -d'|' -f2)"
    detail="$(echo "$rec" | cut -d'|' -f3-)"
    emit_human_row "$name" "$status" "$detail"
  done
  echo ""
  case "$OVERALL" in
    ADMITTED) OC="$GREEN" ;;
    PARTIAL)  OC="$YELLOW" ;;
    BLOCKED)  OC="$RED" ;;
    *)        OC="$CYAN" ;;
  esac
  echo -e "  overall: ${OC}${OVERALL}${NC}"
  echo ""
fi

# ----------------------------------------------------------------------------
# Machine-readable block (JSON)
# ----------------------------------------------------------------------------
json_escape() {
  # Minimal JSON string escaper for backslash and double-quote.
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'
}

echo "-----BEGIN DOCTOR-----"
{
  echo "{"
  echo "  \"tool\": \"lsp-max-doctor\","
  echo "  \"root\": \"$(json_escape "$PROJECT_ROOT")\","
  echo "  \"axes\": ["
  n="${#AXES[@]}"; i=0
  for rec in "${AXES[@]}"; do
    name="$(echo "$rec" | cut -d'|' -f1)"
    status="$(echo "$rec" | cut -d'|' -f2)"
    detail="$(echo "$rec" | cut -d'|' -f3-)"
    i=$(( i + 1 ))
    sep=","
    [ "$i" -eq "$n" ] && sep=""
    echo "    {\"axis\": \"$(json_escape "$name")\", \"status\": \"$(json_escape "$status")\", \"detail\": \"$(json_escape "$detail")\"}$sep"
  done
  echo "  ],"
  echo "  \"overall\": \"$OVERALL\""
  echo "}"
}
echo "-----END DOCTOR-----"

exit $EXIT_CODE
