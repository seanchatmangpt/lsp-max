#!/usr/bin/env bash
#
# scripts/preflight.sh
#
# The "will CI pass?" oracle. Runs EXACTLY the four gates that
# .github/workflows/ci.yml runs, on the pinned nightly toolchain declared in
# rust-toolchain.toml, so that a clear result locally implies the same result
# in CI. Local/CI parity is the entire point: plain `cargo` could resolve to a
# different default toolchain, so every gate is pinned with `cargo +<channel>`.
#
# Gates (mirrored 1:1 from ci.yml, run in CI's evaluation order):
#   1. fmt    cargo fmt -- --check
#   2. clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
#   3. doc    RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features
#   4. test   cargo test --workspace
#
# Output: a per-gate bounded-status table using bounded statuses only.
#
# Exit codes:
#   0 = every gate ADMITTED
#   1 = at least one gate BLOCKED
#   2 = pinned toolchain absent -> parity UNKNOWN (oracle cannot speak for CI)

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# ----------------------------------------------------------------------------
# Resolve the pinned toolchain from rust-toolchain.toml (the CI source of truth).
# ci.yml installs `nightly-2026-04-15`; rust-toolchain.toml pins the same channel.
# We do not hardcode it here so the oracle tracks the file, not a stale copy.
# ----------------------------------------------------------------------------
TOOLCHAIN_FILE="$PROJECT_ROOT/rust-toolchain.toml"
if [ ! -f "$TOOLCHAIN_FILE" ]; then
  echo -e "${RED}rust-toolchain.toml absent at $TOOLCHAIN_FILE${NC}"
  echo -e "Parity oracle status: UNKNOWN (no pinned channel to mirror CI)"
  exit 2
fi

PINNED_CHANNEL="$(
  grep -E '^[[:space:]]*channel[[:space:]]*=' "$TOOLCHAIN_FILE" \
    | head -n 1 \
    | sed -E 's/.*=[[:space:]]*"([^"]+)".*/\1/'
)"

if [ -z "$PINNED_CHANNEL" ]; then
  echo -e "${RED}Could not parse channel from rust-toolchain.toml${NC}"
  echo -e "Parity oracle status: UNKNOWN"
  exit 2
fi

echo -e "${MAGENTA}============================================================${NC}"
echo -e "${BLUE}Preflight: CI parity oracle${NC}"
echo -e "${MAGENTA}============================================================${NC}"
echo -e "Pinned channel (rust-toolchain.toml): ${PINNED_CHANNEL}"
echo -e "Mirroring: .github/workflows/ci.yml gates fmt, clippy, docs, test"
echo ""

# ----------------------------------------------------------------------------
# Verify the pinned toolchain is installed. If it is not, refuse to substitute
# another toolchain: a clear run on the wrong nightly would be a false oracle.
# ----------------------------------------------------------------------------
if ! rustup toolchain list 2>/dev/null | grep -q "$PINNED_CHANNEL"; then
  echo -e "${YELLOW}Pinned toolchain '${PINNED_CHANNEL}' is not installed.${NC}"
  echo -e "${YELLOW}Install it to obtain a CI-faithful result:${NC}"
  echo -e "    rustup toolchain install ${PINNED_CHANNEL}"
  echo ""
  echo -e "Parity oracle status: UNKNOWN (cannot mirror CI on a different toolchain)"
  exit 2
fi

CARGO=(cargo "+${PINNED_CHANNEL}")

# Gate identifiers, in CI's order.
GATES=("fmt" "clippy" "doc" "test")
declare -A GATE_STATUS
declare -A GATE_CMD

GATE_CMD["fmt"]="cargo fmt -- --check"
GATE_CMD["clippy"]="cargo clippy --workspace --all-targets --all-features -- -D warnings"
GATE_CMD["doc"]="RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --all-features"
GATE_CMD["test"]="cargo test --workspace"

FAILED=0

run_gate() {
  local name="$1"
  shift
  echo -e "${BLUE}► [${name}] ${GATE_CMD[$name]}${NC}"
  if "$@"; then
    GATE_STATUS["$name"]="ADMITTED"
  else
    GATE_STATUS["$name"]="BLOCKED"
    FAILED=1
  fi
  echo ""
}

# ----------------------------------------------------------------------------
# Gate 1: fmt  (ci.yml job `fmt`)
# ----------------------------------------------------------------------------
run_gate "fmt" "${CARGO[@]}" fmt -- --check

# ----------------------------------------------------------------------------
# Gate 2: clippy  (ci.yml job `clippy`)
# ----------------------------------------------------------------------------
run_gate "clippy" "${CARGO[@]}" clippy --workspace --all-targets --all-features -- -D warnings

# ----------------------------------------------------------------------------
# Gate 3: doc  (ci.yml job `docs`, RUSTDOCFLAGS="-D warnings")
# ----------------------------------------------------------------------------
echo -e "${BLUE}► [doc] ${GATE_CMD[doc]}${NC}"
if RUSTDOCFLAGS="-D warnings" "${CARGO[@]}" doc --workspace --no-deps --all-features; then
  GATE_STATUS["doc"]="ADMITTED"
else
  GATE_STATUS["doc"]="BLOCKED"
  FAILED=1
fi
echo ""

# ----------------------------------------------------------------------------
# Gate 4: test  (ci.yml job `test`)
# ----------------------------------------------------------------------------
run_gate "test" "${CARGO[@]}" test --workspace

# ----------------------------------------------------------------------------
# Bounded-status table
# ----------------------------------------------------------------------------
echo -e "${MAGENTA}============================================================${NC}"
echo -e "${BLUE}Preflight gate table (channel ${PINNED_CHANNEL})${NC}"
echo -e "${MAGENTA}============================================================${NC}"
printf "%-10s %-12s %s\n" "GATE" "STATUS" "COMMAND"
for g in "${GATES[@]}"; do
  st="${GATE_STATUS[$g]:-UNKNOWN}"
  case "$st" in
    ADMITTED) col="${GREEN}" ;;
    BLOCKED)  col="${RED}" ;;
    *)        col="${YELLOW}" ;;
  esac
  printf "%-10s ${col}%-12s${NC} %s\n" "$g" "$st" "${GATE_CMD[$g]}"
done
echo ""

if [ "$FAILED" -eq 0 ]; then
  echo -e "${GREEN}Preflight verdict: ADMITTED (all four CI gates passed on pinned nightly)${NC}"
  exit 0
else
  echo -e "${RED}Preflight verdict: BLOCKED (one or more CI gates failed)${NC}"
  exit 1
fi
