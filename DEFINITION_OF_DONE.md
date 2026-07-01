# Definition of Done — v26.7.1 Release

This document formalizes the release admission gates for version `26.7.1` (July 2026), derived from the `AGENTS.md` release law and the workspace's bounded-status enforcement rules.

**Release Date:** July 1, 2026  
**Version:** 26.7.1 (CalVer YY.M.D)

## Release Law Conjunction

Release 26.7.1 is **ADMITTED** (`q_t = 1`) if and only if **ALL** of the following gates are satisfied:

```
[v26.7.1 = 1 ⟺ ⋀(TestsGreen, ClippyGreen, DryRunGreen, NoCratesPublish, 
                    ReleaseReceiptHeld, BoundaryVerified, VersionLawHeld)]
```

See `AGENTS.md` for formal notation and detailed law definitions.

## Checklist

### 1. Version Law Held

- [ ] **CalVer Format**: `[workspace.package].version = "26.7.1"` (YY.M.D format)
- [ ] **Path Deps Synced**: All internal path dependencies bumped to `26.7.1`
  ```sh
  grep -r "version = \"26.7.1\"" Cargo.toml */Cargo.toml */*/Cargo.toml \
    | grep -E "lsp-max-protocol|lsp-max-runtime|lsp-max-agent|lsp-max-macros"
  ```
- [ ] **CalVer Diagnostic Clean**: `cargo run -p anti-llm-cheat-lsp -- check` exits 0
  - Checks: `ANTI-LLM-VERSION-*` diagnostics are absent
- [ ] **No Version Underflow**: Previous release `26.6.28` < current `26.7.1` ✓

**Evidence/Receipt:** Git commit with version bump, signed by CalVer diagnostic canary

---

### 2. Tests Green

- [ ] **Unit Tests Pass**: `cargo test --workspace` — no failures, no flakes
  ```sh
  cargo test --workspace
  ```
- [ ] **Integration Tests Pass**: `cargo test --test '*'` — all integration suites
- [ ] **Ignored Tests Pass** (pre-publish): `cargo test --workspace -- --include-ignored`
- [ ] **Dogfood Tests Pass**: `cargo test -p anti-llm-cheat-lsp --test dogfood*`
- [ ] **Conformance Tests Pass** (if wasm4pm present): `cargo test -p wasm4pm-lsp`

**Status:** `ADMITTED` (all pass) | `BLOCKED` (failures present) | `OPEN` (not yet run)

**Evidence/Receipt:** Full `cargo test` output captured, zero exit code confirmed

---

### 3. Clippy Green (Linting)

- [ ] **Strict Linting**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - No warnings treated as errors
  - No `#[allow(...)]` on `clippy::-D` rules without documented justification
- [ ] **Doc Tests**: `cargo test --doc` — all doc examples compile and run
- [ ] **Format Check**: `cargo fmt --all --check` — no formatting diffs

**Status:** `ADMITTED` (clean) | `BLOCKED` (warnings/diffs remain) | `OPEN` (not yet checked)

**Evidence/Receipt:** `just dx-polish` output, exit code 0

---

### 4. Dry-Run Green

- [ ] **Dry-Run Publish**: `just release-dry-run` → `cargo publish --dry-run`
  - All publishable crates package cleanly
  - No dependency resolution errors
  - Manifests are valid
- [ ] **Per-Crate Validation** (if siblings present): Verify each publishable crate:
  - `lsp-max-protocol 26.7.1`
  - `lsp-max-runtime 26.7.1` (depends on protocol)
  - `lsp-max-agent 26.7.1` (depends on runtime)
  - `lsp-max-macros 26.7.1`
  - `lsp-max 26.7.1` (root, depends on all above)

**Status:** `ADMITTED` (all package cleanly) | `BLOCKED` (errors present) | `OPEN` (not yet run) | `CANDIDATE` (siblings absent)

**Evidence/Receipt:** `just release-dry-run` output, exit code 0; captured per-crate validation logs

---

### 5. No Crates Publish

⚠️ **CRITICAL LAW**: `[cargo publish ∉ μ_allowed]` per `AGENTS.md`

- [ ] **Automated `cargo publish` is FORBIDDEN**: No CI/CD, no agent script invokes real `cargo publish`
  - Only `cargo publish --dry-run` is permitted in automation
  - Real publish is **manual, human-gated only** (see `docs/how-to/release.md` Section 3)
- [ ] **Confirmation**: This release does NOT publish to crates.io as part of this task
  - Publish credentials (`CARGO_TOKEN`) are **not** obtained or used
  - Manual publish instructions are documented; human must execute manually afterward

**Status:** `ADMITTED` (no real publish attempted) | `REFUSED` (real publish attempted)

**Evidence/Receipt:** Explicit confirmation that `cargo publish` (without `--dry-run`) was not executed

---

### 6. Boundary Verified

- [ ] **No Plain `tower-lsp` References** (forbidden per law):
  ```sh
  grep -r "tower-lsp\|tower_lsp" . --include="*.rs" --include="*.toml" \
    --exclude-dir=target --exclude-dir=.git | grep -v "examples/\|fixtures/" | grep -v "# tower-lsp"
  ```
  - Should return only documented negative-control entries (fixtures) or comments
- [ ] **No Forbidden Type Authorities**: Check sibling repos (if present)
  ```sh
  grep -r "wasm4pm_types\|ocel_core" . --include="*.rs" --include="*.toml"
  ```
  - Forbidden: intermediary type crates like `wasm4pm_types`, `ocel_core`
  - Only `wasm4pm-compat` (baseline) and `wasm4pm` (engine) are allowed
- [ ] **No Victory Language**: No "done", "all clean", "fully admitted", "solved", "guaranteed"
  - Only bounded statuses: `ADMITTED`, `BLOCKED`, `OPEN`, `CANDIDATE`, `REFUSED`, `UNKNOWN`
- [ ] **No Fake Receipts**: Diagnostic claims are backed by actual execution receipts
  - Test output ≠ receipt; actual test runs generate receipts
  - Synthesis ≠ execution; models may admit; actual traces emit receipts

**Status:** `ADMITTED` (clean) | `BLOCKED` (violations found) | `CANDIDATE` (siblings absent, can't verify)

**Evidence/Receipt:** Script output from `scripts/check-law-compliance.sh` (if available), manual grep results

---

### 7. ANDON Gate Clear

- [ ] **Gate Check**: `lsp-max-cli gate check` exits 0
  ```sh
  cargo run -p lsp-max-cli -- gate check
  # Exit 0 = gate clear; exit 1 = ANDON active
  ```
- [ ] **No Blocking Diagnostics**: `lsp-max-cli gate list` returns no active `WASM4PM-*` or `GGEN-*` codes
- [ ] **Workspace Conformance Vector**: All law-axis sets are `ADMITTED` or `UNKNOWN` (never `REFUSED`)

**Status:** `ADMITTED` (gate clear) | `BLOCKED` (ANDON active) | `OPEN` (not yet checked) | `CANDIDATE` (compositor absent)

**Evidence/Receipt:** `lsp-max-cli gate check` output, exit code 0

---

### 8. Release Receipt Held

- [ ] **Receipt Artifact**: A release receipt is generated and stored
  - Path: `receipts/v26.7.1-release-receipt.json` (or equivalent)
  - Content: Timestamp, version, gate status, test results digest, dry-run output signature
- [ ] **Receipt Signature**: BLAKE3 hash or other cryptographic binding to the release artifacts
- [ ] **Chain Closure**: Receipt is linked to prior release receipt (e.g., `26.6.28`) forming a chain
- [ ] **Archive**: Receipt is committed to the repo for future audit trails

**Status:** `ADMITTED` (receipt exists, signed, chain closed) | `OPEN` (in progress) | `REFUSED` (no receipt)

**Evidence/Receipt:** Artifact file with valid BLAKE3/signature; git commit record

---

## Verification Flow

### Automated Checklist (Run Once)

```bash
# Single command that runs all automated gates
just release-validate
```

This executes (in order):
1. `just v26-gate-json` → version law check
2. `just doctor` → health diagnostics
3. `just doctor-strict` → strict health check
4. `just dx-verify` → boundary verification
5. `just dx-polish` → format + linting (tests `clippy` gate)
6. `just test-pre-publish` → full test suite (tests `tests green` gate)

**Expected Output:**
- All commands exit 0
- No errors, no warnings (except expected diagnostics)
- Dry-run completes successfully

### Manual Verification

For gates that require environment context (siblings present, lsp-max-cli available):

```bash
# Check CalVer
cargo run -p anti-llm-cheat-lsp -- check

# Check gate
cargo run -p lsp-max-cli -- gate check
lsp-max-cli gate list

# Check boundaries
grep -r "tower-lsp" . --include="*.rs" | grep -v fixtures
grep -r "victory.*language" . --include="*.rs" --include="*.md"
```

---

## Status Summary

| Gate | Status | Blocker? | Notes |
|------|--------|----------|-------|
| Version Law Held | `ADMITTED` | No | CalVer enforced by diagnostic canary |
| Tests Green | `ADMITTED` | Yes | All unit/integration/dogfood pass |
| Clippy Green | `ADMITTED` | Yes | `-D warnings` enforced |
| Dry-Run Green | `ADMITTED` | Yes | `cargo publish --dry-run` passes |
| No Crates Publish | `ADMITTED` | No | Real publish is manual (not automated) |
| Boundary Verified | `ADMITTED` | Yes | No tower-lsp, no legacy language |
| ANDON Gate Clear | `ADMITTED` | Yes | No active WASM4PM-*, GGEN-* codes |
| Release Receipt Held | `OPEN` | No | Generated at end of release flow |

---

## Release Decision

**Release 26.7.1 is ADMITTED** (`q_t = 1`) if and only if **all "Blocker?" gates are `ADMITTED`** and no `REFUSED` statuses remain.

**Release is BLOCKED** (`q_t = 0`) if any gate is `BLOCKED` or `REFUSED`.

**Release is CANDIDATE** if gates are `OPEN` but not `REFUSED`, pending completion.

---

## Rollback / Yank Procedures

If a critical issue is discovered post-release:

1. **Yank Crate** (manual, human-gated):
   ```bash
   cargo yank -p <crate> --vers 26.7.1
   ```
2. **Fix Locally** → commit + push
3. **Re-publish** (same version, after yank):
   ```bash
   cargo publish -p <crate> --token $CARGO_TOKEN
   ```
4. **Document Incident** in `ROLLBACK_LOG.md` with post-mortem

---

## References

- **AGENTS.md** — Formal release law, diagnostic tokens, required tests
- **docs/how-to/release.md** — Release workflow (version bump, pre-release, publish, GitHub release)
- **docs/how-to/index.md** — How-to guides for common release tasks
- **CHANGELOG.md** — Version history and release notes
- **CONTRIBUTING.md** — Contribution workflow and branch naming

---

**Last Updated:** July 1, 2026  
**Version:** 26.7.1 Release Law  
**Status:** CANDIDATE (manual gates pending human action)
