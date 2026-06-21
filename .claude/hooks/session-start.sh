#!/bin/bash
set -euo pipefail

# Only run in remote (Claude Code on the web) sessions.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

REPO_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
PARENT_DIR="$(dirname "$REPO_DIR")"

echo "==> lsp-max phase-shift bootstrap"
echo "    repo:   $REPO_DIR"
echo "    parent: $PARENT_DIR"

# ── 1. Sibling repos ────────────────────────────────────────────────────────
clone_if_missing() {
  local name="$1"
  local url="$2"
  local target="$PARENT_DIR/$name"
  if [ -d "$target/.git" ]; then
    echo "==> $name: already present"
  else
    echo "==> Cloning $name ..."
    if git clone --depth=1 "$url" "$target" 2>&1 | tail -2; then
      echo "==> $name: cloned"
    else
      echo "WARN: $name clone failed (non-fatal)"
    fi
  fi
}

clone_if_missing "wasm4pm"        "https://github.com/seanchatmangpt/wasm4pm.git"
clone_if_missing "wasm4pm-compat" "https://github.com/seanchatmangpt/wasm4pm-compat.git"

if [ ! -d "$PARENT_DIR/lsp-types-max" ]; then
  echo "WARN: lsp-types-max absent at $PARENT_DIR/lsp-types-max — build will be partial"
fi

# ── 2. Persistent session environment ───────────────────────────────────────
ENV_FILE="${CLAUDE_ENV_FILE:-/dev/null}"

# Default mesh state into /tmp so lsp-max-cli can bootstrap without write errors.
echo "LSP_MAX_STATE_PATH=/tmp/lsp-max-mesh-state.json" >> "$ENV_FILE"

# Use lld for fast linking when available (5-10× vs system ld on this image).
if command -v lld >/dev/null 2>&1; then
  echo "RUSTFLAGS=-C linker=clang -C link-arg=-fuse-ld=lld" >> "$ENV_FILE"
  echo "==> lld linker enabled"
fi

echo "CARGO_BUILD_JOBS=4" >> "$ENV_FILE"

# Expose the lsp-max-cli debug binary once it is built.
echo "PATH=$REPO_DIR/target/debug:${PATH}" >> "$ENV_FILE"

# ── 3. Build lsp-max-cli (best-effort; requires all three sibling repos) ───
ALL_DEPS_PRESENT=true
for dep in lsp-types-max wasm4pm wasm4pm-compat; do
  if [ ! -d "$PARENT_DIR/$dep" ]; then
    ALL_DEPS_PRESENT=false
    echo "==> Missing sibling: $dep — skipping lsp-max-cli build"
  fi
done

if [ "$ALL_DEPS_PRESENT" = "true" ]; then
  echo "==> All sibling deps present — building lsp-max-cli ..."
  cd "$REPO_DIR"
  if cargo build -p lsp-max-cli 2>&1 | tail -5; then
    echo "==> lsp-max-cli: CANDIDATE"
  else
    echo "WARN: lsp-max-cli build failed — gate check will pass-through"
  fi
else
  echo "==> Sibling deps incomplete — lsp-max-cli build skipped"
fi

echo "==> Phase-shift bootstrap complete"
