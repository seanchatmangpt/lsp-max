# Review Specification: wasm4pm-compat Sibling Repository

**Purpose**: Validate wasm4pm-compat as the sole baseline type authority for process-mining types in lsp-max.

**Status**: BLOCKED (repo not available in this environment)

**Scope**: Complete architectural and code quality review

---

## Critical Architecture Constraints

From CLAUDE.md (architectural mandate):

> **"Architectural mandate (enforced by `just dx-verify`, which greps the sibling repos): no intermediary type crates (`wasm4pm_types`, `ocel_core` are forbidden), and the words `legacy`/`deprecated`/`shim`/`facade`/`backward compatibility` must not appear anywhere in `../wasm4pm-compat` or `../wasm4pm` — including in comments you write."**

### Constraint 1: Solo Type Authority

**Requirement**: wasm4pm-compat must be the **sole baseline type authority**.

**What this means**:
- ✅ Core type definitions (OCEL, process models, breed types) live here
- ✅ Consumers (lsp-max, wasm4pm) import and use these types
- ❌ No intermediary type crates (wasm4pm_types, ocel_core, etc.)
- ❌ No duplicate type definitions across repos
- ❌ No type wrapper layers (prevents layering)

**Review Checklist**:

```
[ ] Scan Cargo.toml: Is there a `wasm4pm_types` crate?
    - [ ] YES → FAIL: Violates solo authority
    - [ ] NO → PASS

[ ] Scan Cargo.toml: Is there an `ocel_core` crate?
    - [ ] YES → FAIL: Violates solo authority  
    - [ ] NO → PASS

[ ] Scan Cargo.toml: Are any type crates published?
    - [ ] YES → Check if consumers have other sources
    - [ ] NO → PASS

[ ] Check imports in lsp-max:
    - Command: grep -r "from_ocel" /home/user/lsp-max/src
    - Are these importing from wasm4pm-compat or elsewhere?
    - [ ] All from wasm4pm-compat → PASS
    - [ ] Mix of sources → FAIL

[ ] Check imports in wasm4pm:
    - Command: grep -r "OCEL::" ../wasm4pm/
    - Are these importing from wasm4pm-compat?
    - [ ] All from wasm4pm-compat → PASS
    - [ ] Self-defined types → FAIL
```

---

### Constraint 2: No Forbidden Language

**Requirement**: No `legacy`, `deprecated`, `shim`, `facade`, or `backward compatibility` language.

**Why**: These words signal violation of clean architecture. If type authority needs wrappers/adapters, it's broken.

**Review Checklist**:

```
[ ] Full codebase grep for forbidden words:
    grep -r "legacy" ../wasm4pm-compat/
    grep -r "deprecated" ../wasm4pm-compat/
    grep -r "shim" ../wasm4pm-compat/
    grep -r "facade" ../wasm4pm-compat/
    grep -r "backward compatib" ../wasm4pm-compat/
    
    - [ ] ZERO matches → PASS
    - [ ] Any matches → FAIL (list them)

[ ] Check comments for hedging language:
    grep -r "workaround\|hack\|TODO.*remove\|FIXME" ../wasm4pm-compat/ | head -20
    
    - [ ] Few or none → PASS
    - [ ] Many → FAIL (indicates debt)

[ ] Check module names for "compat":
    find ../wasm4pm-compat/ -name "*compat*"
    find ../wasm4pm-compat/ -name "*adapter*"
    
    - [ ] ZERO → PASS (we're already in the compat layer)
    - [ ] Any → SUSPICIOUS (nested compat layers)
```

---

## Integration Points with lsp-max

### Usage: fresh_names::FRESH_NAME_PAIRS

**File**: `src/diagnostics/cognition_laws.rs:24`

```rust
use wasm4pm_compat::fresh_names::FRESH_NAME_PAIRS;
```

**What it does**: Provides a mapping of Oracle identifiers to Fresh names (e.g., for A8 adversary detection).

**Review**:

```
[ ] Does FRESH_NAME_PAIRS exist and export correctly?
    - [ ] YES → PASS
    - [ ] NO → FAIL

[ ] Is FRESH_NAME_PAIRS feature-gated or conditional?
    - [ ] NO (unconditional) → PASS (always available)
    - [ ] YES → Check if conditionals are justified
    
[ ] Is FRESH_NAME_PAIRS mutable or lazy-initialized?
    - [ ] Immutable const/static → PASS
    - [ ] Lazy-static → Check init cost
    - [ ] Mutable → FAIL (violates purity)

[ ] Performance: Is fresh_names loaded on every diagnostic run?
    - [ ] YES but it's O(1) lookup → PASS
    - [ ] YES and expensive → FAIL (should cache at session start)
    - [ ] NO (lazy-loaded) → PASS
```

---

## Type Exports Audit

**Requirement**: All types needed by consumers are exported and stable.

**Known consumers**: 
- lsp-max (root)
- lsp-max-runtime
- crates/wasm4pm-lsp
- crates/gc005-wasm4pm-adapter
- examples/wasm4pm-compat-lsp

**Review Checklist**:

```
[ ] Scan Cargo.toml [package] section:
    - [ ] Has `publish = true`? (should be publishable)
    - [ ] Has `repository` URL?
    - [ ] Has `license`?
    - [ ] Version is CalVer (26.6.9 or similar)?

[ ] Scan lib.rs for public module tree:
    pub mod ocel;
    pub mod models;
    pub mod breeds;
    pub mod fresh_names;
    
    - [ ] Modules are clearly organized?
    - [ ] No `pub mod internal` or `pub mod _private`?

[ ] Stability: Check if types are versioned or feature-gated:
    - [ ] Types marked #[doc(hidden)]? → Count them
    - [ ] Types behind feature flags? → List them
    - [ ] Goal: All consumer-facing types are public and stable

[ ] OCEL Type Completeness:
    - [ ] Do types cover OCEL 1.0 standard?
    - [ ] Are there gaps (missing event attributes, object properties)?
    - [ ] Do generated code (from metaModel.json) match these types?
```

---

## Anti-LLM Violations Audit

Run anti-llm-cheat-lsp against wasm4pm-compat:

```bash
cargo run --example anti-llm-cheat-lsp -- --scan --dir ../wasm4pm-compat
```

**Expected Results**:

```
[ ] Zero SURFACE-* violations (no tower-lsp references)
[ ] Zero CLAIM-* violations (no victory language)
[ ] Zero RECEIPT-* violations (test results have receipts)
[ ] Zero ROUTE-* violations (execution is traced)
[ ] Zero VERSION-* violations (uses CalVer)
[ ] Zero CHEAT-* violations (no hardcoded metrics)
[ ] Zero LEGACY-type violations (no forbidden language)
```

---

## Code Quality Gates

### Clippy

```bash
cd ../wasm4pm-compat && cargo clippy --all-targets --all-features -- -D warnings
```

**Expected**: ZERO warnings

**Review**:

```
[ ] Clippy clean?
    - [ ] YES → PASS
    - [ ] NO → List violations
    
[ ] Any `#[allow(clippy::*)]`?
    - [ ] ZERO → PASS (no suppression)
    - [ ] YES → Justify each one
```

### Formatting

```bash
cd ../wasm4pm-compat && cargo fmt --all -- --check
```

**Expected**: Already formatted

```
[ ] Formatted correctly?
    - [ ] YES → PASS
    - [ ] NO → FAIL (run `cargo fmt`)
```

---

## Test Coverage

**Requirement**: All public types have tests (direct + transitive via lsp-max).

**Review Checklist**:

```
[ ] OCEL type tests:
    - [ ] Serialization/deserialization roundtrip?
    - [ ] Schema validation?
    - [ ] Invalid states rejected?
    - [ ] Edge cases (empty, huge, malformed)?

[ ] Fresh-names tests:
    - [ ] FRESH_NAME_PAIRS complete?
    - [ ] No collisions in mapping?
    - [ ] Handles all Oracle classes (A8–A12)?

[ ] Breed type tests:
    - [ ] Breed instantiation?
    - [ ] Registry parsing?
    - [ ] Invalid breed configs rejected?

[ ] Integration tests:
    - [ ] `cargo test -p wasm4pm-compat`
    - [ ] All tests pass?
    - [ ] Coverage > 70%?

[ ] Dogfood tests in lsp-max:
    - [ ] `cargo test -p gc005-wasm4pm-adapter --test dogfood_*`
    - [ ] Do these validate wasm4pm-compat behavior?
```

---

## Documentation

**Requirement**: Public types are documented; all modules have doc comments.

**Review Checklist**:

```
[ ] lib.rs has module-level doc comment?
    - [ ] YES → PASS
    - [ ] NO → FAIL

[ ] Each public type has doc comment?
    - Scan: grep -c "^/// " ../wasm4pm-compat/src/*.rs
    - Goal: All public items documented
    
[ ] Examples in doc comments?
    - [ ] OCEL types have /// # Examples?
    - [ ] fresh_names has usage examples?
    - [ ] Breed types document instantiation?

[ ] Links to external specs?
    - [ ] OCEL 1.0 spec linked?
    - [ ] Process mining terminology?
    - [ ] Breed format description?

[ ] README.md exists?
    - [ ] YES → Read it
    - [ ] NO → Should create one
```

---

## Dependency Audit

**Requirement**: Minimal, stable dependencies. No internal wasm4pm interdependencies (must go through lsp-max or wasm4pm).

**Review Checklist**:

```
[ ] Scan Cargo.toml [dependencies]:
    - [ ] Any `path = "../wasm4pm"` deps?
        → FAIL (creates circular dependency)
    - [ ] Any version mismatches with lsp-max/Cargo.toml?
        → FAIL (version conflicts)

[ ] Check feature flags:
    - [ ] Are critical features documented?
    - [ ] Do defaults make sense?
    - [ ] Any cyclic feature dependencies?

[ ] Transitive dependencies:
    - [ ] `cargo tree` output reasonable?
    - [ ] Any unmaintained crates?
    - [ ] Duplicate versions of same crate?
```

---

## Build Matrix

**Requirement**: Builds on all feature combinations.

**Review Checklist**:

```bash
# All-features
[ ] cargo build --all-features
    - [ ] SUCCESS → PASS
    - [ ] FAIL → List errors

# No features
[ ] cargo build --no-default-features
    - [ ] SUCCESS → PASS
    - [ ] FAIL → List errors

# Individual features
for feature in $(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name=="wasm4pm-compat") | .features | keys | .[]'); do
  [ ] cargo build --features "$feature"
      - [ ] SUCCESS
      - [ ] FAIL → List errors
done
```

---

## Compatibility Matrix

**Requirement**: Must work across:
- Rust MSRV (from lsp-max's CLAUDE.md: 1.82.0)
- Platforms: Linux, macOS, Windows

**Review Checklist**:

```
[ ] Rust MSRV test:
    - [ ] cargo build --rust-version 1.82.0 succeeds?
    - [ ] YES → PASS
    - [ ] NO → Fix or update MSRV

[ ] Platform-specific code:
    - [ ] Any `#[cfg(target_os = "...")]`?
    - [ ] Justified and tested?
    
[ ] Endianness:
    - [ ] Serialization handles both?
    - [ ] Byte order specified?
```

---

## Cross-Repo Validation

**Requirement**: Spot-check that lsp-max can build with wasm4pm-compat.

**Review Checklist**:

```bash
[ ] cargo check -p lsp-max
    - [ ] SUCCESS → PASS
    - [ ] FAIL → List errors

[ ] cargo test -p lsp-max --test "*cognition*"
    - [ ] SUCCESS → PASS
    - [ ] FAIL → Diagnose

[ ] cargo build -p wasm4pm-lsp
    - [ ] SUCCESS → PASS
    - [ ] FAIL → Diagnose

[ ] cargo build --example wasm4pm-compat-lsp
    - [ ] SUCCESS → PASS
    - [ ] FAIL → Diagnose
```

---

## Summary Scorecard

| Category | Weight | Status | Notes |
|----------|--------|--------|-------|
| **Solo Authority** | 30% | ? | No intermediary type crates, FRESH_NAME_PAIRS works |
| **Clean Code** | 20% | ? | Clippy clean, formatted, no forbidden language |
| **Testing** | 20% | ? | Unit + integration, coverage > 70% |
| **Documentation** | 10% | ? | All public types documented |
| **Dependencies** | 10% | ? | Minimal, stable, no cycles |
| **Build Matrix** | 10% | ? | All features, Rust MSRV, cross-platform |

**Overall**: Status = **BLOCKED** (repo not available)

---

## Next Steps

1. **Clone wasm4pm-compat** into `../wasm4pm-compat`
2. **Run checks above** using provided commands
3. **Document findings** in this checklist
4. **Open issues** for any failures
5. **Gate**: Must pass all checks before lsp-max merges

---

## References

- CLAUDE.md: Architectural mandate (solo type authority, forbidden language)
- cognition_laws.rs: Usage of FRESH_NAME_PAIRS
- Cargo.toml [patch.crates-io]: Patch declaration for wasm4pm-compat

---

**Status**: CANDIDATE (review spec complete; awaiting repo access)
