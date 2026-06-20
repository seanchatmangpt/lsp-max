#!/usr/bin/env bash
#
# scripts/status-report.sh
#
# Project-health surface for the lsp-max law-state runtime. ONE command that
# renders, at a glance, where the runtime stands across its governing axes:
#
#   - ANDON / gate state        (lsp-max-cli gate check exit; UNKNOWN if absent)
#   - Law compliance            (reuses scripts/check-law-compliance.sh)
#   - Doc coverage              (reads DOC_COVERAGE_LOG.md; never appends)
#   - LSP 3.18 surface          (counts from on-disk inventory + delta matrix)
#   - Workspace crate count     (parsed from root Cargo.toml [workspace].members)
#   - Sibling versions          (../lsp-types-max ../wasm4pm-compat ../wasm4pm)
#
# Every metric carries a bounded status. The surface is READ-ONLY: it observes
# and reports; it never mutates a tracked file. The closing posture line is a
# bounded aggregate, never a victory claim, and never collapses UNKNOWN into
# ADMITTED or REFUSED.
#
# Usage:
#   scripts/status-report.sh            # human-readable table
#   scripts/status-report.sh --json     # machine-readable JSON block
#
# Bounded statuses only: ADMITTED, CANDIDATE, BLOCKED, REFUSED, UNKNOWN,
# PARTIAL, OPEN. The LSP 3.18 conformance verdict is intentionally UNKNOWN
# here: a truthful verdict requires the live extractor's on-disk evidence
# scan, which this cheap surface does not perform. The surface count is a
# count, not a verdict — it is never reported as conformance.

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

JSON_MODE=0
if [ "${1:-}" = "--json" ]; then
  JSON_MODE=1
fi

# Color a bounded status token for the human table.
status_color() {
  case "$1" in
    ADMITTED) printf '%b' "${GREEN}" ;;
    PARTIAL | CANDIDATE | OPEN) printf '%b' "${YELLOW}" ;;
    BLOCKED | REFUSED) printf '%b' "${RED}" ;;
    *) printf '%b' "${CYAN}" ;; # UNKNOWN and anything else
  esac
}

# JSON string escaper for values that may contain quotes/backslashes.
json_escape() {
  local s="${1:-}"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  printf '%s' "$s"
}

# ============================================================================
# Metric 1 — ANDON / gate state
# ============================================================================
# Prefer the built lsp-max-cli binary; fall back to cargo run. If neither the
# binary exists nor the compositor wrote a gate file, the state is UNKNOWN —
# absence of a gate file means the compositor is not running (not enforced),
# which is a genuine gap, not a clear gate.
gate_status="UNKNOWN"
gate_detail="lsp-max-cli not located; gate state not observed"

GATE_BIN=""
if command -v lsp-max-cli >/dev/null 2>&1; then
  GATE_BIN="lsp-max-cli"
elif [ -x "target/debug/lsp-max-cli" ]; then
  GATE_BIN="target/debug/lsp-max-cli"
elif [ -x "target/release/lsp-max-cli" ]; then
  GATE_BIN="target/release/lsp-max-cli"
fi

if [ -n "$GATE_BIN" ]; then
  gate_out="$("$GATE_BIN" gate check 2>&1)" && gate_rc=0 || gate_rc=$?
  if [ "$gate_rc" -eq 0 ]; then
    if printf '%s' "$gate_out" | grep -qi 'compositor_active.*false\|not located\|absent'; then
      gate_status="UNKNOWN"
      gate_detail="gate file absent; compositor not running (not enforced)"
    else
      gate_status="ADMITTED"
      gate_detail="gate clear (exit 0)"
    fi
  else
    gate_status="BLOCKED"
    gate_detail="ANDON active (exit ${gate_rc})"
  fi
fi

# ============================================================================
# Metric 2 — Law compliance
# ============================================================================
# Reuse the canonical scanner. Exit 0 -> ADMITTED, exit 1 -> BLOCKED, any
# other outcome (script missing / not executable) -> UNKNOWN.
law_status="UNKNOWN"
law_detail="scripts/check-law-compliance.sh not located"

if [ -f "scripts/check-law-compliance.sh" ]; then
  if bash scripts/check-law-compliance.sh >/dev/null 2>&1; then
    law_status="ADMITTED"
    law_detail="no plain tower references, no victory language detected"
  else
    law_rc=$?
    if [ "$law_rc" -eq 1 ]; then
      law_status="BLOCKED"
      law_detail="compliance violation(s) reported by scanner"
    else
      law_status="UNKNOWN"
      law_detail="scanner exited ${law_rc} (could not observe)"
    fi
  fi
fi

# ============================================================================
# Metric 3 — Doc coverage (read-only)
# ============================================================================
# Read the most recent bijection ratio + status from DOC_COVERAGE_LOG.md. This
# does NOT run update-doc-coverage.sh (that script appends to the log); the
# surface is read-only. If the most recent iteration carries no machine-parsable
# ratio line (the recent iterations are narrative), the ratio is UNKNOWN while
# the file's last bounded Status token is still surfaced if present.
doc_status="UNKNOWN"
doc_detail="DOC_COVERAGE_LOG.md not located"

if [ -f "DOC_COVERAGE_LOG.md" ]; then
  doc_ratio="$(grep -oE '\*\*Ratio\*\*: [0-9]+\.[0-9]+' DOC_COVERAGE_LOG.md | tail -n1 | grep -oE '[0-9]+\.[0-9]+' || true)"
  doc_token="$(grep -oE '\*\*Status\*\*: (ADMITTED|CANDIDATE|BLOCKED|PARTIAL|OPEN|UNKNOWN)' DOC_COVERAGE_LOG.md | tail -n1 | grep -oE '(ADMITTED|CANDIDATE|BLOCKED|PARTIAL|OPEN|UNKNOWN)' || true)"
  iters="$(grep -cE '^## Iteration' DOC_COVERAGE_LOG.md || true)"
  if [ -n "$doc_token" ]; then
    doc_status="$doc_token"
  else
    # File present but the latest entry is narrative (no Status token). The
    # ratio is unobserved from a machine-parsable field — do not fabricate one.
    doc_status="UNKNOWN"
  fi
  if [ -n "$doc_ratio" ]; then
    doc_detail="last bijection ratio ${doc_ratio} over ${iters:-0} iteration(s)"
  else
    doc_detail="${iters:-0} iteration(s) logged; latest entry is narrative (ratio UNKNOWN)"
  fi
fi

# ============================================================================
# Metric 4 — LSP 3.18 surface (counts, NOT a conformance verdict)
# ============================================================================
# Cheap, honest counts from on-disk artifacts:
#   - enumerated method surface (lsp318_message_inventory.json)
#   - delta-changelog feature rows (rules/lsp318.rs LSP318-### identifiers)
# The conformance VERDICT is deliberately UNKNOWN: deriving it truthfully
# requires the live extractor's evidence scan (handler wired + transcript on
# disk + receipt), which this surface does not run. Reporting the row count as
# "covered" would assert ChangelogCoverage(15) => SpecCoverage(3.18), the exact
# implication ANTI-LLM-LSP318-COMB-001 refutes. So: counts reported, verdict UNKNOWN.
lsp318_status="UNKNOWN"
lsp318_methods="0"
lsp318_rows="0"
lsp318_detail="LSP 3.18 surface artifacts not located"

INV="crates/anti-llm-cheat-lsp/generated/lsp318_message_inventory.json"
ROWS_SRC="crates/anti-llm-cheat-lsp/src/rules/lsp318.rs"
have_lsp318=0
if [ -f "$INV" ]; then
  # Count "method" keys without a JSON parser dependency.
  lsp318_methods="$(grep -cE '"method"\s*:' "$INV" || echo 0)"
  have_lsp318=1
fi
if [ -f "$ROWS_SRC" ]; then
  lsp318_rows="$(grep -oE 'LSP318-[0-9]+' "$ROWS_SRC" | sort -u | wc -l | tr -d ' ')"
  have_lsp318=1
fi
if [ "$have_lsp318" -eq 1 ]; then
  # Surface is enumerated (a count exists); the conformance verdict remains
  # UNKNOWN by construction — the count is not a verdict.
  lsp318_status="UNKNOWN"
  lsp318_detail="${lsp318_methods} methods enumerated, ${lsp318_rows} delta rows; conformance verdict UNKNOWN (needs live extractor)"
fi

# ============================================================================
# Metric 5 — Workspace crate count
# ============================================================================
# Parse the [workspace].members array from the root Cargo.toml. Count quoted
# member entries between `members = [` and the closing `]`.
crate_count="0"
crate_status="UNKNOWN"
crate_detail="Cargo.toml [workspace].members not located"

if [ -f "Cargo.toml" ]; then
  crate_count="$(awk '
    /^\[workspace\]/ { inws=1 }
    inws && /members[[:space:]]*=[[:space:]]*\[/ { inmem=1 }
    inmem && /"/ { for (i=1;i<=NF;i++) if ($i ~ /"/) c++ }
    inmem && /\]/ { inmem=0; inws=0 }
    END { print c+0 }
  ' Cargo.toml)"
  if [ "${crate_count:-0}" -gt 0 ]; then
    crate_status="ADMITTED"
    crate_detail="${crate_count} workspace members declared"
  fi
fi

# ============================================================================
# Metric 6 — Workspace version (CalVer)
# ============================================================================
ws_version="UNKNOWN"
if [ -f "Cargo.toml" ]; then
  ws_version="$(awk '
    /^\[workspace\.package\]/ { inwp=1; next }
    inwp && /^\[/ { inwp=0 }
    inwp && /^[[:space:]]*version[[:space:]]*=/ {
      gsub(/[^0-9.]/,""); print; exit
    }
  ' Cargo.toml)"
  [ -z "$ws_version" ] && ws_version="UNKNOWN"
fi

# ============================================================================
# Metric 7 — Sibling versions (build prerequisites)
# ============================================================================
# Read the first `version = "X.Y.Z"` line from each sibling Cargo.toml. ABSENT
# (UNKNOWN) when the checkout is missing — the workspace does not build without
# these, so their absence is a reportable gap, not a pass.
sibling_names=("lsp-types-max" "wasm4pm-compat" "wasm4pm")
sibling_paths=("../lsp-types-max" "../wasm4pm-compat" "../wasm4pm")
sibling_versions=()
sibling_statuses=()
siblings_present=0
for i in "${!sibling_paths[@]}"; do
  p="${sibling_paths[$i]}/Cargo.toml"
  if [ -f "$p" ]; then
    v="$(grep -m1 -oE 'version[[:space:]]*=[[:space:]]*"[0-9][^"]*"' "$p" | grep -oE '[0-9][0-9.]*' | head -n1 || true)"
    if [ -n "$v" ]; then
      sibling_versions+=("$v")
      sibling_statuses+=("ADMITTED")
      siblings_present=$((siblings_present + 1))
    else
      sibling_versions+=("UNKNOWN")
      sibling_statuses+=("UNKNOWN")
    fi
  else
    sibling_versions+=("ABSENT")
    sibling_statuses+=("UNKNOWN")
  fi
done

if [ "$siblings_present" -eq "${#sibling_paths[@]}" ]; then
  sibling_overall="ADMITTED"
  sibling_detail="all ${siblings_present} sibling checkouts present"
elif [ "$siblings_present" -eq 0 ]; then
  sibling_overall="UNKNOWN"
  sibling_detail="no sibling checkouts located (workspace will not build)"
else
  sibling_overall="PARTIAL"
  sibling_detail="${siblings_present}/${#sibling_paths[@]} sibling checkouts present"
fi

# ============================================================================
# Overall posture (bounded aggregate; never victorious; UNKNOWN never collapses)
# ============================================================================
# Precedence, strongest signal first:
#   any BLOCKED/REFUSED -> BLOCKED   (a hard gap dominates)
#   else any UNKNOWN    -> PARTIAL   (observed signals are clear, gaps remain)
#   else any PARTIAL/CANDIDATE/OPEN -> PARTIAL
#   else                -> ADMITTED  (every observed axis ADMITTED; still bounded)
all_status=("$gate_status" "$law_status" "$doc_status" "$lsp318_status" "$crate_status" "$sibling_overall")
posture="ADMITTED"
has_unknown=0
has_soft=0
for s in "${all_status[@]}"; do
  case "$s" in
    BLOCKED | REFUSED) posture="BLOCKED" ;;
    UNKNOWN) has_unknown=1 ;;
    PARTIAL | CANDIDATE | OPEN) has_soft=1 ;;
  esac
done
if [ "$posture" != "BLOCKED" ]; then
  if [ "$has_unknown" -eq 1 ] || [ "$has_soft" -eq 1 ]; then
    posture="PARTIAL"
  fi
fi

posture_note="bounded aggregate over 6 axes; UNKNOWN is held distinct, not collapsed"

# ============================================================================
# Render
# ============================================================================
if [ "$JSON_MODE" -eq 1 ]; then
  # Build sibling JSON array.
  sib_json=""
  for i in "${!sibling_names[@]}"; do
    [ -n "$sib_json" ] && sib_json="${sib_json},"
    sib_json="${sib_json}{\"name\":\"$(json_escape "${sibling_names[$i]}")\",\"version\":\"$(json_escape "${sibling_versions[$i]}")\",\"status\":\"${sibling_statuses[$i]}\"}"
  done

  cat <<EOF
{
  "report": "lsp-max-status",
  "generated_utc": "$(date -u '+%Y-%m-%dT%H:%M:%SZ')",
  "workspace_version": "$(json_escape "$ws_version")",
  "metrics": {
    "gate": { "status": "${gate_status}", "detail": "$(json_escape "$gate_detail")" },
    "law_compliance": { "status": "${law_status}", "detail": "$(json_escape "$law_detail")" },
    "doc_coverage": { "status": "${doc_status}", "detail": "$(json_escape "$doc_detail")" },
    "lsp318_surface": { "status": "${lsp318_status}", "methods_enumerated": ${lsp318_methods:-0}, "delta_rows": ${lsp318_rows:-0}, "detail": "$(json_escape "$lsp318_detail")" },
    "workspace_crates": { "status": "${crate_status}", "count": ${crate_count:-0}, "detail": "$(json_escape "$crate_detail")" },
    "siblings": { "status": "${sibling_overall}", "detail": "$(json_escape "$sibling_detail")", "checkouts": [${sib_json}] }
  },
  "posture": "${posture}",
  "posture_note": "$(json_escape "$posture_note")"
}
EOF
  # Exit code mirrors posture (same contract as the human path): BLOCKED -> 1,
  # otherwise 0. Not a pass claim — a bounded CI signal. The JSON block is fully
  # written before exit, so consumers (incl. the `report` noun) still parse it.
  if [ "$posture" = "BLOCKED" ]; then
    exit 1
  fi
  exit 0
fi

# Human table.
echo -e "${MAGENTA}════════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  lsp-max — Law-State Runtime Status Surface${NC}"
echo -e "${MAGENTA}════════════════════════════════════════════════════════════════${NC}"
echo -e "  ${BLUE}workspace version${NC} : ${ws_version}  ${BLUE}(CalVer)${NC}"
echo -e "  ${BLUE}generated (UTC)${NC}   : $(date -u '+%Y-%m-%d %H:%M:%S')"
echo

printf '  %-20s  %-10s  %s\n' "AXIS" "STATUS" "DETAIL"
printf '  %-20s  %-10s  %s\n' "--------------------" "----------" "----------------------------------------"

render_row() {
  local axis="$1" st="$2" detail="$3"
  printf '  %-20s  ' "$axis"
  printf '%b%-10s%b  ' "$(status_color "$st")" "$st" "${NC}"
  printf '%s\n' "$detail"
}

render_row "ANDON / gate"      "$gate_status"      "$gate_detail"
render_row "law compliance"    "$law_status"       "$law_detail"
render_row "doc coverage"      "$doc_status"       "$doc_detail"
render_row "LSP 3.18 surface"  "$lsp318_status"    "$lsp318_detail"
render_row "workspace crates"  "$crate_status"     "$crate_detail"
render_row "siblings"          "$sibling_overall"  "$sibling_detail"

echo
echo -e "  ${BLUE}sibling checkouts${NC}"
for i in "${!sibling_names[@]}"; do
  printf '    %-18s  ' "${sibling_names[$i]}"
  printf '%b%-10s%b  ' "$(status_color "${sibling_statuses[$i]}")" "${sibling_statuses[$i]}" "${NC}"
  printf '%s\n' "${sibling_versions[$i]}"
done

echo
echo -e "${MAGENTA}────────────────────────────────────────────────────────────────${NC}"
printf '  %s ' "POSTURE:"
printf '%b%s%b\n' "$(status_color "$posture")" "$posture" "${NC}"
echo -e "  ${BLUE}${posture_note}${NC}"
echo -e "${MAGENTA}════════════════════════════════════════════════════════════════${NC}"

# Exit code mirrors posture for CI consumption without asserting victory:
#   BLOCKED -> 1 (a hard gap is present), otherwise 0 (bounded, not a pass claim).
if [ "$posture" = "BLOCKED" ]; then
  exit 1
fi
exit 0
