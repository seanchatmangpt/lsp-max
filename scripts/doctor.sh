#!/usr/bin/env bash
# Full-spectrum read-only health oracle for the lsp-max workspace.
# Each check pairs a bounded status with the exact one-line fix. It never
# compiles and never mutates. `--json` emits one machine-readable object
# (for agents / CI); otherwise a human table. Exit 1 only when overall BLOCKED.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PARENT="$(cd "$ROOT/.." && pwd)"
JSON=0
[ "${1:-}" = "--json" ] && JSON=1

PIN="$(awk -F'"' '/channel/{print $2; exit}' "$ROOT/rust-toolchain.toml" 2>/dev/null)"

declare -a ROWS # each: id|status|detail|fix
add() { ROWS+=("$1|$2|$3|${4:-}"); }

# Rust toolchain pin match
active="$(rustup show active-toolchain 2>/dev/null | awk '{print $1}')"
if [ -z "$active" ]; then
  add toolchain UNKNOWN "rustup not reporting an active toolchain" "rustup show"
elif [ -n "$PIN" ] && [[ "$active" == *"$PIN"* ]]; then
  add toolchain ADMITTED "$active" ""
else
  add toolchain PARTIAL "active=$active pin=$PIN" "rustup toolchain install $PIN"
fi

# Sibling repos: present? version? working tree clean? (dirty = gc006 release blocker)
for s in lsp-types-max wasm4pm-compat wasm4pm; do
  d="$PARENT/$s"
  if [ ! -d "$d" ]; then
    add "sibling:$s" BLOCKED "MISSING" "just setup"
    continue
  fi
  ver="$(awk -F'"' '/^version[[:space:]]*=/{print $2; exit}' "$d/Cargo.toml" 2>/dev/null)"
  [ -z "$ver" ] && ver="?"
  dirty="$(git -C "$d" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
  if [ "${dirty:-0}" -eq 0 ]; then
    add "sibling:$s" ADMITTED "v$ver clean" ""
  else
    add "sibling:$s" PARTIAL "v$ver ${dirty} uncommitted (gc006 release blocker)" "git -C $d status"
  fi
done

# Build-readiness: resolve the dependency graph without compiling
if cargo metadata --no-deps --format-version 1 >/dev/null 2>&1; then
  add resolve ADMITTED "cargo metadata resolves the workspace" ""
else
  add resolve BLOCKED "workspace does not resolve (siblings?)" "just setup"
fi

# ANDON gate: only via an already-built binary — the doctor never triggers a compile
gate_bin=""
command -v lsp-max-cli >/dev/null 2>&1 && gate_bin="lsp-max-cli"
if [ -z "$gate_bin" ]; then
  for c in "$ROOT/target/debug/lsp-max-cli" "$ROOT/target/release/lsp-max-cli"; do
    [ -x "$c" ] && gate_bin="$c" && break
  done
fi
if [ -n "$gate_bin" ]; then
  if "$gate_bin" gate check >/dev/null 2>&1; then
    add gate ADMITTED "ANDON CLEAR" ""
  else
    add gate BLOCKED "ANDON SET" "$gate_bin diagnostics snapshot"
  fi
else
  add gate UNKNOWN "lsp-max-cli not built (cannot read gate)" "cargo build -p lsp-max-cli"
fi

# Config completeness for canonical keys (see docs/lsp-max-config-keys.md)
cfg="${LSP_MAX_CONFIG:-$HOME/.lsp-max-config.json}"
for k in api_base model; do
  if [ -f "$cfg" ] && grep -q "\"$k\"" "$cfg" 2>/dev/null; then
    add "config:$k" ADMITTED "set in $cfg" ""
  else
    add "config:$k" PARTIAL "using built-in default" "lsp-max-cli config set $k <value>"
  fi
done

# target/ disk footprint (informational)
sz="$(du -sm "$ROOT/target" 2>/dev/null | awk '{print $1}')"
add disk OPEN "${sz:-0}MB in target/" "just qol-clean"

# Overall bounded status (BLOCKED dominates; PARTIAL/UNKNOWN demote ADMITTED)
overall=ADMITTED
for r in "${ROWS[@]}"; do
  st="$(printf '%s' "$r" | cut -d'|' -f2)"
  [ "$st" = BLOCKED ] && overall=BLOCKED
  { [ "$st" = PARTIAL ] || [ "$st" = UNKNOWN ]; } && [ "$overall" = ADMITTED ] && overall=PARTIAL
done

if [ "$JSON" -eq 1 ]; then
  printf '{"overall":"%s","checks":[' "$overall"
  first=1
  for r in "${ROWS[@]}"; do
    IFS='|' read -r id st de fx <<<"$r"
    de=${de//\\/\\\\}; de=${de//\"/\\\"}
    fx=${fx//\\/\\\\}; fx=${fx//\"/\\\"}
    [ $first -eq 0 ] && printf ','
    first=0
    printf '{"id":"%s","status":"%s","detail":"%s","fix":"%s"}' "$id" "$st" "$de" "$fx"
  done
  printf ']}\n'
else
  printf '  %-18s %-9s %s\n' "CHECK" "STATUS" "DETAIL"
  for r in "${ROWS[@]}"; do
    IFS='|' read -r id st de fx <<<"$r"
    printf '  %-18s %-9s %s%s\n' "$id" "$st" "$de" "${fx:+   -> $fx}"
  done
  printf '\n  overall: %s\n' "$overall"
fi

[ "$overall" = BLOCKED ] && exit 1
exit 0
