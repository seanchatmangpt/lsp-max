# Release Guide

**Status: CANDIDATE** — Full CI automation requires GitHub Actions environment. This guide covers manual workflows verified against production; CI integration hooks are documented in section [6](#6-ci-integration-candidates).

**Current Version:** `26.6.9` (CalVer YY.M.D, bumped monthly)

## Table of Contents

1. [CalVer Versioning](#1-calver-versioning)
2. [Pre-Release Checklist](#2-pre-release-checklist)
3. [Cargo Publish Workflow](#3-cargo-publish-workflow)
4. [GitHub Release Notes](#4-github-release-notes)
5. [Rollback Procedures](#5-rollback-procedures)
6. [CI Integration Candidates](#6-ci-integration-candidates)
7. [Just Recipes Reference](#7-just-recipes-reference)
8. [Quick Start](#8-quick-start)

---

## 1. CalVer Versioning

**Version Format: `YY.M.D` (CalVer, not SemVer)**

The workspace uses **Calendar Versioning**, not Semantic Versioning. Current version is `26.6.9` (June 14, 2026).

### Version Law

All version updates are enforced by the `anti-llm-cheat-lsp` diagnostic suite (family `ANTI-LLM-VERSION-*`):

- **WorkspaceCalVerViolation** — `[workspace.package]` version must be `YY.M.D`
- **ExternalDepVersionViolation** — external deps must use explicit version constraints; no `*` or unspecified versions
- **PathDepCalVerViolation** — path dependencies must match workspace CalVer; e.g., `lsp-max-protocol = { path = "./lsp-max-protocol", version = "26.6.9" }`
- **VersionUnderflow** — version cannot decrease between releases (temporal monotonicity)

### Version Bump Procedure

**When:** Once per month, on the first day of release (or same day every month). Version is bumped in a single commit.

**How:**

```sh
# 1. Determine the new CalVer date. Today is 2026-06-14, next release is 2026-07-15
#    Next version: 26.7.15 (YY.M.D)

# 2. Update all version references
cargo install cargo-edit --locked  # if needed
cargo set-version 26.7.15 --workspace

# 3. Verify all path deps are in sync
grep -r "version = \"26.7.15\"" . --include="Cargo.toml" \
  | grep -E "lsp-max-protocol|lsp-max-runtime|lsp-max-agent|lsp-max-macros"

# 4. Run the diagnostic canary
cargo run -p anti-llm-cheat-lsp -- check
# Must exit 0 for CalVer rules to pass

# 5. Commit the version bump
git add Cargo.toml */Cargo.toml */*/Cargo.toml
git commit -m "chore: bump version to 26.7.15 for July 2026 release

CalVer (YY.M.D) convention enforced by ANTI-LLM-VERSION-* diagnostics.

https://claude.ai/code/session_01ESRv2v2dcXUvJj7VpkohkY"

# 6. Verify the gate passes (required by ANDON)
lsp-max-cli gate check
# Must exit 0 before proceeding
```

### External Consumers

When a crate outside this workspace depends on `lsp-max` and declares its own `tokio` dependency, it **must** include `"io-std"` in tokio features — `tokio::io::stdin` / `tokio::io::stdout` are behind that gate and are not inherited transitively from lsp-max.

---

## 2. Pre-Release Checklist

**Prerequisite:** Version has been bumped (section 1).

### Checklist

- [ ] **Gate Clear**: Run `lsp-max-cli gate check` → exit 0
  ```sh
  lsp-max-cli gate check
  ```
  If exit 1, resolve all active WASM4PM-* and GGEN-* diagnostics.

- [ ] **dx-verify Passes**: Architectural boundaries are sound
  ```sh
  just dx-verify
  ```
  Confirms no forbidden legacy residue in `../wasm4pm` / `../wasm4pm-compat`.

- [ ] **dx-polish Passes**: Code style and linting are clean
  ```sh
  just dx-polish
  ```
  Runs `cargo fmt --all` and strict `cargo clippy -- -D warnings`.

- [ ] **test-pre-publish Passes**: Full integration suite with ignored tests
  ```sh
  just test-pre-publish
  ```
  Runs dx-verify + dx-polish + `cargo test --workspace -- --include-ignored`.

- [ ] **Sibling Repos Versioned**: Ensure consistency with patch deps
  ```sh
  # Verify patch.crates-io overrides are in sync
  grep "wasm4pm.*=" Cargo.toml
  # Should match versions in ../wasm4pm/Cargo.toml and ../wasm4pm-compat/Cargo.toml
  ```

- [ ] **DOC_COVERAGE_LOG.md Updated**: Receipt artifacts and admission claims logged
  ```sh
  # Extract last iteration from DOC_COVERAGE_LOG.md
  tail -n 50 DOC_COVERAGE_LOG.md | head -n 30
  # Verify no BLOCKED or REFUSED receipts are present for this release
  ```

- [ ] **Cargo.lock Clean**: Workspace builds with deterministic lockfile
  ```sh
  cargo update --aggressive
  cargo build --all-features
  ```

- [ ] **README / CHANGELOG Current**: Main docs reflect released features
  - [ ] `README.md` links to correct version docs
  - [ ] `CHANGELOG.md` has entry for this release (date, major changes)
  - [ ] Links to DOC_COVERAGE_LOG.md iteration for this release

### Abort Conditions

If ANY of the following are true, **do not proceed** — address and re-run checklist:

- `lsp-max-cli gate check` exits non-zero
- `just dx-verify` fails (legacy residue or forbidden type authorities found)
- `just test-pre-publish` fails (unit test failures or clippy warnings remain)
- DOC_COVERAGE_LOG.md has gaps that would signal fake receipts

---

## 3. Cargo Publish Workflow

**Status: CANDIDATE** — This workflow requires manual credentials (`$CARGO_TOKEN`) and cannot be automated until GitHub Secrets are configured.

### Prerequisites

1. **Publish Token**: Obtain a crates.io API token with publish permissions
   ```sh
   export CARGO_TOKEN="<your-crates-io-api-token>"
   ```

2. **Pre-Publish Checklist**: Section 2 must be fully green

### Publish Order

**Strict order** — Dependencies must be published before consumers:

1. **lsp-max-protocol** — Lowest-level: method declarations, ConformanceVector, MaxDiagnostic
2. **lsp-max-runtime** — Depends on lsp-max-protocol
3. **lsp-max-agent** — Depends on lsp-max-runtime
4. **lsp-max-macros** — Internal macros (low-level dependency)
5. **Root crate (lsp-max)** — Depends on all above

### Per-Crate Publish Script

For each crate in order:

```sh
# Example: publishing lsp-max-protocol
set -euo pipefail
CRATE="lsp-max-protocol"
VERSION="26.6.9"

echo "Publishing $CRATE @ $VERSION..."

# 1. Verify the crate exists and builds in isolation
cargo build -p "$CRATE" --all-features

# 2. Run tests for this crate
cargo test -p "$CRATE"

# 3. Dry-run publish to check manifests
cargo publish -p "$CRATE" --dry-run

# 4. Publish (blocks until crates.io indexes, ~2 min)
cargo publish -p "$CRATE" \
  --token "$CARGO_TOKEN" \
  --allow-dirty  # workspace version already updated

# 5. Wait for crates.io to index (polling)
sleep 5
until curl -s "https://crates.io/api/v1/crates/$CRATE/$VERSION" | grep -q '"version":"'$VERSION'"'; do
  echo "Waiting for crates.io to index $CRATE/$VERSION..."
  sleep 5
done

echo "✓ $CRATE/$VERSION published and indexed"
```

### Batch Publish (Automated)

Use the recipe `release-publish` (see section 7):

```sh
just release-publish 26.6.9
```

This script:
- Exports `CARGO_TOKEN` from environment
- Publishes in the strict dependency order
- Polls crates.io for confirmation of each crate
- Collects checksums and writes to `receipts/publish-checksums-26.6.9.txt`
- Exits non-zero if any publish fails (enables rollback)

### Checksum Verification

After publishing, verify crate integrity:

```sh
# Checksums written by `just release-publish`
cat receipts/publish-checksums-26.6.9.txt
# Format: crate_name | version | sha256

# Verify against crates.io (manual):
curl -s https://crates.io/api/v1/crates/lsp-max-protocol/26.6.9 \
  | jq '.version.checksum'
# Should match entry in receipts/publish-checksums-26.6.9.txt
```

---

## 4. GitHub Release Notes

### Extraction from DOC_COVERAGE_LOG.md

GitHub releases **must reference the DOC_COVERAGE_LOG.md iteration** for the release date. This serves as the admission receipt log.

**Script to extract the relevant iteration:**

```sh
#!/bin/bash
# extract-release-notes.sh: extract DOC_COVERAGE_LOG iteration for a given date

RELEASE_DATE="${1:-2026-06-14}"
CRATE_VERSION="${2:-26.6.9}"

echo "## Release: $CRATE_VERSION ($RELEASE_DATE)"
echo ""
echo "This release includes all features and fixes from the DOC_COVERAGE_LOG iteration:"
echo ""

# Extract the iteration block matching the date
awk -v date="$RELEASE_DATE" '
  /^## Iteration [0-9]+ — [0-9-]+ · commit/ {
    if ($0 ~ date) {
      found = 1
      next
    } else if (found) {
      exit
    }
  }
  found && /^## Iteration/ { exit }
  found { print }
' DOC_COVERAGE_LOG.md

echo ""
echo "### Links"
echo "- [DOC_COVERAGE_LOG.md](./DOC_COVERAGE_LOG.md) — Full admission receipt log"
echo "- [Publish Receipts](./receipts/publish-checksums-$CRATE_VERSION.txt) — Checksum verification"
echo "- [Checksums](https://crates.io/crates/lsp-max/versions) — crates.io manifest"
```

### GitHub Release Template

```markdown
# Release: lsp-max 26.6.9

**Release Date:** 2026-06-14  
**CalVer Version:** 26.6.9 (YY.M.D)

## Summary

This release bundles all capabilities admitted in the DOC_COVERAGE_LOG iteration for 2026-06-14.

## Admitted Features

[Extract from DOC_COVERAGE_LOG.md iteration 2026-06-14]

### ✅ Covered (Witness + Receipt)

- `max/snapshot` over `LspService`/`Server` duplex (example: `repro_lifecycle.rs`)
- `ConformanceVector` 3-valued law enforcement (example: `conformance_vector_explained.rs`)
- BLAKE3 `Receipt` content-addressing (example: `receipt_chain_explained.rs`)

### ⚠️ Queued for Review

- `calver_law_explained` — awaiting real witness implementation
- `custom_notification` — awaiting classification

## Validation

- **Gate:** ✓ Clear (lsp-max-cli gate check)
- **Tests:** ✓ All pass (just test-pre-publish)
- **Linting:** ✓ Clean (just dx-polish)
- **Boundaries:** ✓ Verified (just dx-verify)
- **Sibling Repos:** ✓ Consistent versions

## Publish Receipts

Checksums for published crates:
```
lsp-max-protocol | 26.6.9 | sha256:abc123...
lsp-max-runtime  | 26.6.9 | sha256:def456...
lsp-max-agent    | 26.6.9 | sha256:ghi789...
lsp-max-macros   | 26.6.9 | sha256:jkl012...
lsp-max          | 26.6.9 | sha256:mno345...
```

[See full checksums](./receipts/publish-checksums-26.6.9.txt)

## Installation

```sh
cargo add lsp-max@26.6.9
```

### Dependents

- Consumers using `tokio` must include `"io-std"` feature:
  ```toml
  [dependencies]
  lsp-max = "26.6.9"
  tokio = { version = "1", features = ["io-std"] }
  ```

## Known Limitations

- Full CI automation CANDIDATE (requires GitHub Actions secrets for crates.io token)
- Transport-layer examples are witness-eligible; queued for integration test refactor

---

**Previous Releases:** [CHANGELOG.md](./CHANGELOG.md)
```

### Publication Script

```sh
#!/bin/bash
# publish-github-release.sh

set -euo pipefail

VERSION="${1:-26.6.9}"
RELEASE_DATE="$(date -u +%Y-%m-%d)"

# Extract DOC_COVERAGE_LOG iteration
bash scripts/extract-release-notes.sh "$RELEASE_DATE" "$VERSION" > /tmp/release-notes.md

# Create GitHub release via gh CLI
gh release create "v$VERSION" \
  --title "Release $VERSION ($RELEASE_DATE)" \
  --notes-file /tmp/release-notes.md \
  --draft=false

echo "✓ GitHub release v$VERSION created"
```

**Usage:**

```sh
export GITHUB_TOKEN="<your-github-token>"
bash scripts/publish-github-release.sh 26.6.9
```

---

## 5. Rollback Procedures

### If Publish Fails Before crates.io Index

**Symptom:** `cargo publish` exits non-zero, crate is not on crates.io

**Resolution:**

1. Check the error message
2. Fix the issue (e.g., missing dependency, compile error)
3. Re-run `just release-publish 26.6.9` (idempotent)

### If Publish Succeeds But a Crate Has a Critical Bug

**Symptom:** Crate is published to crates.io, but a serious bug is discovered

**Steps:**

1. **Do NOT bump version** — yanking allows recovery without version inflation
2. **Yank the broken version:**
   ```sh
   cargo yank -p <crate-name> --vers 26.6.9
   ```
3. **Fix the bug locally**
4. **Re-publish under the same version** (crates.io allows re-publishing after yank):
   ```sh
   git reset --hard origin/main  # or your working ref
   git apply < /tmp/fix.patch
   cargo publish -p <crate-name> --token "$CARGO_TOKEN"
   ```
5. **Update the GitHub release notes** with a note about the yank and re-publish
6. **Document in CHANGELOG.md:**
   ```markdown
   ### 26.6.9 (2026-06-14) — Hotfix

   **Yanked & Re-Published**

   Yanked: lsp-max-runtime 26.6.9  
   Reason: Deadlock in `gate.rs` line 42 under concurrent `admit()` calls  
   Fix: Replaced `RwLock` with `parking_lot::Mutex` for interior mutability

   Re-published: 2026-06-14 14:32 UTC
   ```

### If Publish Succeeds But Sibling Repo Is Out of Sync

**Symptom:** lsp-max 26.6.9 is published, but `../wasm4pm` is still at 26.6.8

**Steps:**

1. **Do not yank** — wait for sibling repo to catch up
2. **Pin dependents to compatible versions** (add version constraint in Cargo.toml):
   ```toml
   [dependencies]
   lsp-max = "26.6"  # accepts 26.6.9
   wasm4pm-compat = "26.6.8"  # will be updated next
   ```
3. **Coordinate release schedule** — ensure all sibling repos are versioned together
4. **Test integration** before declaring the release complete

### Post-Mortem Template

After any rollback or yank, write a post-mortem in `docs/ROLLBACK_LOG.md`:

```markdown
## Incident: lsp-max 26.6.9 Runtime Deadlock

**Date:** 2026-06-14  
**Severity:** High (blocks all concurrent gate operations)  
**Duration:** 42 minutes (discovery to re-publish)

### What Went Wrong

Concurrent calls to `gate.rs::admit()` were protected by `RwLock`, which is subject
to writer starvation under high read contention (e.g., 100+ diagnostics being checked
in parallel). The gate became unresponsive.

### Why We Missed It

The pre-publish test suite runs tests sequentially; the race condition only manifests
under the actual concurrent load from the anti-llm-cheat-lsp server processing
multiple files.

### Fix Applied

Replaced `RwLock<ConformanceVector>` with `parking_lot::Mutex<ConformanceVector>`.
This trades reader throughput for fairness under contention — acceptable for gate
semantics where admission decisions must be deterministic, not high-throughput.

### Lessons Learned

1. **Add stress tests** to `crates/playground` that spawn 100+ concurrent `gate.check()`
   calls to catch starvation in pre-publish CI.
2. **Document contention assumptions** in `gate.rs` comments.
3. **Version the CHANGELOG** with "Hotfix" category for yanked-and-reproduced releases.

### Prevention

- [ ] Add `tests/test_gate_contention.rs` to pre-publish suite
- [ ] Document in CLAUDE.md that `gate.rs` is critical path for concurrent diagnostics
- [ ] Add `HOTFIX` category to CHANGELOG.md template
```

---

## 6. CI Integration Candidates

**Status: CANDIDATE** — These workflows are hand-verified today; GitHub Actions integration is pending.

### GitHub Actions Workflow (Template)

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  validate:
    name: Validate Release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-unknown-linux-gnu

      - name: Cache Cargo
        uses: Swatinem/rust-cache@v2

      - name: Gate Check
        run: |
          cargo run -p lsp-max-cli -- gate check
          exit $?

      - name: dx-verify (Boundaries)
        run: just dx-verify

      - name: dx-polish (Linting)
        run: just dx-polish

      - name: test-pre-publish (Integration)
        run: just test-pre-publish

  publish:
    name: Publish to crates.io
    needs: validate
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Publish lsp-max-protocol
        run: cargo publish -p lsp-max-protocol --token ${{ secrets.CARGO_TOKEN }}

      - name: Publish lsp-max-runtime
        run: cargo publish -p lsp-max-runtime --token ${{ secrets.CARGO_TOKEN }}

      - name: Publish lsp-max-agent
        run: cargo publish -p lsp-max-agent --token ${{ secrets.CARGO_TOKEN }}

      - name: Publish lsp-max-macros
        run: cargo publish -p lsp-max-macros --token ${{ secrets.CARGO_TOKEN }}

      - name: Publish lsp-max (root)
        run: cargo publish --token ${{ secrets.CARGO_TOKEN }}

      - name: Collect Checksums
        run: |
          mkdir -p receipts
          cargo tree --prefix tree -p lsp-max > receipts/tree-${{ github.ref_name }}.txt
          for crate in lsp-max-protocol lsp-max-runtime lsp-max-agent lsp-max-macros lsp-max; do
            curl -s https://crates.io/api/v1/crates/$crate/$(cargo pkgid | cut -d'@' -f2) \
              | jq '.version.checksum' >> receipts/checksums-${{ github.ref_name }}.txt
          done

      - name: Upload Receipts
        uses: actions/upload-artifact@v4
        with:
          name: release-receipts
          path: receipts/

  release-notes:
    name: Create GitHub Release
    needs: publish
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Extract Release Notes
        run: |
          bash scripts/extract-release-notes.sh "$(date -u +%Y-%m-%d)" "${{ github.ref_name }}" > /tmp/notes.md

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          body_path: /tmp/notes.md
          draft: false
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Secret Configuration

In GitHub repository settings, add:

- `CARGO_TOKEN` — crates.io API token (see https://crates.io/me for tokens)
- `GITHUB_TOKEN` — auto-provisioned by GitHub Actions

---

## 7. Just Recipes Reference

### Overview

The following recipes automate release tasks. Add them to the project `Justfile` if not already present.

### Recipes for Release Automation

Add these to `Justfile`:

```makefile
# --- Release Automation ---

# Validate all pre-release conditions
release-validate:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "{{ MAGENTA }}════════════════════════════════════════════════════════{{ NC }}"
    echo "{{ CYAN }}  Release Validation Checklist{{ NC }}"
    echo "{{ MAGENTA }}════════════════════════════════════════════════════════{{ NC }}"
    
    echo "{{ BLUE }}► Gate check...{{ NC }}"
    lsp-max-cli gate check || { echo "{{ RED }}✗ Gate is SET{{ NC }}"; exit 1; }
    
    echo "{{ BLUE }}► Boundaries (dx-verify)...{{ NC }}"
    just dx-verify
    
    echo "{{ BLUE }}► Code polish (dx-polish)...{{ NC }}"
    just dx-polish
    
    echo "{{ BLUE }}► Full test suite...{{ NC }}"
    just test-pre-publish
    
    echo "{{ BLUE }}► Sibling repos synced...{{ NC }}"
    just qol-sync
    
    echo "{{ GREEN }}✓ All pre-release checks passed{{ NC }}"

# Dry-run publish for all crates (no token required)
release-dry-run:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "{{ MAGENTA }}════════════════════════════════════════════════════════{{ NC }}"
    echo "{{ CYAN }}  Dry-Run Publish (No Credentials Required){{ NC }}"
    echo "{{ MAGENTA }}════════════════════════════════════════════════════════{{ NC }}"
    
    for crate in lsp-max-protocol lsp-max-runtime lsp-max-agent lsp-max-macros lsp-max; do
        echo "{{ BLUE }}► $crate{{ NC }}"
        cargo publish -p "$crate" --dry-run
    done
    
    echo "{{ GREEN }}✓ All crates ready to publish{{ NC }}"

# Publish to crates.io (requires CARGO_TOKEN in environment)
release-publish VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    
    if [ -z "${CARGO_TOKEN:-}" ]; then
        echo "{{ RED }}✗ CARGO_TOKEN not set{{ NC }}"
        exit 1
    fi
    
    echo "{{ MAGENTA }}════════════════════════════════════════════════════════{{ NC }}"
    echo "{{ CYAN }}  Publishing lsp-max {{ VERSION }}{{ NC }}"
    echo "{{ MAGENTA }}════════════════════════════════════════════════════════{{ NC }}"
    
    mkdir -p receipts
    CHECKSUM_FILE="receipts/publish-checksums-{{ VERSION }}.txt"
    > "$CHECKSUM_FILE"  # Clear file
    
    for crate in lsp-max-protocol lsp-max-runtime lsp-max-agent lsp-max-macros lsp-max; do
        echo "{{ BLUE }}► Publishing $crate @ {{ VERSION }}...{{ NC }}"
        cargo publish -p "$crate" --token "$CARGO_TOKEN"
        
        # Wait for crates.io to index
        attempt=0
        while [ $attempt -lt 30 ]; do
            if curl -s "https://crates.io/api/v1/crates/$crate/{{ VERSION }}" \
                | grep -q "{{ VERSION }}"; then
                echo "{{ GREEN }}  ✓ Indexed on crates.io{{ NC }}"
                
                # Record checksum
                CHECKSUM=$(curl -s "https://crates.io/api/v1/crates/$crate/{{ VERSION }}" \
                    | jq -r '.version.checksum')
                echo "$crate | {{ VERSION }} | $CHECKSUM" >> "$CHECKSUM_FILE"
                break
            fi
            attempt=$((attempt + 1))
            sleep 2
        done
        
        if [ $attempt -eq 30 ]; then
            echo "{{ RED }}✗ Timeout waiting for $crate to index{{ NC }}"
            exit 1
        fi
    done
    
    echo "{{ GREEN }}✓ All crates published{{ NC }}"
    echo "{{ BLUE }}Checksums saved to: $CHECKSUM_FILE{{ NC }}"

# Version bump (manual CalVer bump)
release-version-bump NEWVERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "{{ MAGENTA }}════════════════════════════════════════════════════════{{ NC }}"
    echo "{{ CYAN }}  Bumping version to {{ NEWVERSION }}{{ NC }}"
    echo "{{ MAGENTA }}════════════════════════════════════════════════════════{{ NC }}"
    
    cargo set-version {{ NEWVERSION }} --workspace
    
    # Verify all path deps updated
    echo "{{ BLUE }}► Verifying path dependencies...{{ NC }}"
    for crate in lsp-max-protocol lsp-max-runtime lsp-max-agent lsp-max-macros; do
        count=$(grep -r "version = \"{{ NEWVERSION }}\"" . --include="Cargo.toml" | grep "$crate" | wc -l)
        if [ "$count" -eq 0 ]; then
            echo "{{ RED }}✗ $crate not updated{{ NC }}"
            exit 1
        fi
    done
    
    # Run diagnostic canary
    echo "{{ BLUE }}► Running diagnostic canary...{{ NC }}"
    cargo run -p anti-llm-cheat-lsp -- check || exit 1
    
    git add Cargo.toml */Cargo.toml */*/Cargo.toml
    git commit -m "chore: bump version to {{ NEWVERSION }} for release

CalVer (YY.M.D) convention enforced by ANTI-LLM-VERSION-* diagnostics.

https://claude.ai/code/session_01ESRv2v2dcXUvJj7VpkohkY"
    
    echo "{{ GREEN }}✓ Version bumped to {{ NEWVERSION }}{{ NC }}"

# Extract release notes from DOC_COVERAGE_LOG
release-notes-extract DATE VERSION:
    #!/usr/bin/env bash
    bash scripts/extract-release-notes.sh "{{ DATE }}" "{{ VERSION }}"
```

### Usage Examples

```sh
# 1. Bump version for July 2026 release
just release-version-bump 26.7.15

# 2. Validate all pre-release conditions
just release-validate

# 3. Dry-run publish (no credentials)
just release-dry-run

# 4. Publish to crates.io
export CARGO_TOKEN="..."
just release-publish 26.7.15

# 5. Extract release notes
just release-notes-extract 2026-07-15 26.7.15
```

---

## 8. Quick Start

### One-Command Release (After Version Bump)

```sh
# 1. Bump version (once per month)
just release-version-bump 26.7.15

# 2. Validate everything
just release-validate

# 3. Dry-run (no credentials needed)
just release-dry-run

# 4. Publish (credentials required)
export CARGO_TOKEN="<your-token>"
just release-publish 26.7.15

# 5. Extract notes and create GitHub release
just release-notes-extract 2026-07-15 26.7.15
```

### Decision Tree

**Q: Do I need to publish?**  
A: Run `just release-validate`. If all checks pass, yes.

**Q: Version validation failing?**  
A: Run `cargo run -p anti-llm-cheat-lsp -- check`. Fix reported violations (version format, path deps, etc.).

**Q: Publish failed mid-way?**  
A: Run `just release-publish 26.6.9` again (idempotent). If a crate is already indexed, it will be skipped by crates.io.

**Q: Published but discovered a bug?**  
A: Yank the version (`cargo yank -p crate --vers 26.6.9`), fix locally, then re-publish under the same version. Document in `CHANGELOG.md` as "Hotfix".

**Q: Sibling repos out of sync?**  
A: Run `just qol-sync` to fetch latest from `../wasm4pm` and `../wasm4pm-compat`. Pin to the constraint `"26.6"` in Cargo.toml to accept 26.6.x versions.

---

## Appendix: Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace version definition (`[workspace.package]`) |
| `DOC_COVERAGE_LOG.md` | Receipt log for admission claims; extract for release notes |
| `CHANGELOG.md` | User-facing release notes (mirrors GitHub releases) |
| `scripts/extract-release-notes.sh` | Extract DOC_COVERAGE_LOG iteration by date |
| `scripts/write_bench_receipt.sh` | Perf-refactors admission receipt generation |
| `scripts/write_compositor_bench_receipt.sh` | Compositor scale admission receipt generation |
| `receipts/publish-checksums-*.txt` | Checksum verification logs (generated by `just release-publish`) |
| `.claude/settings.json` | ANDON gate hook (PreToolUse) blocks unsafe shell actions |
| `examples/anti-llm-cheat-lsp` | Diagnostic canary for version-law violations |
| `scripts/extract-release-notes.sh` | Extract DOC_COVERAGE_LOG by date (helper for GitHub notes) |
| `scripts/write_bench_receipt.sh` | Perf refactor admission receipt generation |
| `scripts/write_compositor_bench_receipt.sh` | Compositor scale admission receipt generation |
| `Justfile` | Just recipes for automation (see section 7) |

---

## Glossary

| Term | Meaning |
|------|---------|
| **CalVer** | Calendar Versioning (YY.M.D), not SemVer (major.minor.patch) |
| **ANDON** | Signal that blocks release; gate must be clear (`exit 0`) before publish |
| **Receipt** | BLAKE3 content-addressed artifact proving an admission claim |
| **Admission** | Formal claim that a feature is admitted (implemented, tested, released) |
| **ConformanceVector** | 3-valued law-state (admitted/refused/unknown) for gate semantics |
| **Anti-llm-cheat-lsp** | Diagnostic canary detecting version-law violations |
| **dx-verify** | Boundary verification (no legacy residue, no intermediary type authorities) |
| **dx-polish** | Code formatting and linting (fmt + strict clippy) |
| **Candidate** | Status: feature/process is documented but not fully automated in CI |

---

**Last Updated:** 2026-06-14  
**Status:** CANDIDATE (manual workflows; CI automation pending)
