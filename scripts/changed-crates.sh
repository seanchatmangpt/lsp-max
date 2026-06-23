#!/usr/bin/env bash
# Print the workspace package names affected by changes since <base> (default HEAD),
# so the inner loop can fmt/clippy/test only what changed.
#   __ALL__  -> a manifest/Justfile/toolchain change means no safe narrowing
#   (empty)  -> no attributable changed crates (UNKNOWN-safe: never guesses)
# This is a fast pre-filter, NOT the admission authority — the full gate
# (dx-polish / CI) still runs the whole workspace with --all-features.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

base="${1:-HEAD}"
files="$( {
  git diff --name-only --diff-filter=d "$base" 2>/dev/null
  git diff --name-only --diff-filter=d --cached 2>/dev/null
  git ls-files --others --exclude-standard 2>/dev/null
} | sort -u )"

[ -z "$files" ] && exit 0

# Manifest/lockfile/Justfile/toolchain changes can affect feature unification
# across the whole workspace — signal a full run rather than narrow unsafely.
if printf '%s\n' "$files" | grep -qE '(^|/)Cargo\.(toml|lock)$|^Justfile$|^rust-toolchain\.toml$'; then
  echo "__ALL__"
  exit 0
fi

meta="$(cargo metadata --no-deps --format-version 1 2>/dev/null)"
[ -z "$meta" ] && { echo "__ALL__"; exit 0; }

printf '%s\n' "$files" | while read -r f; do
  [ -n "$f" ] || continue
  jq -r --arg f "$PWD/$f" \
    '.packages[] | select(($f | startswith(.manifest_path | rtrimstr("Cargo.toml")))) | .name' \
    <<<"$meta"
done | sort -u
