# Contributing to lsp-max

This chapter describes how to contribute code, report issues, and participate in the project.

## Code of Conduct

By participating in this project, you agree to abide by the Code of Conduct in `CODE_OF_CONDUCT.md`.

## Workflow

### Setting Up Your Environment

```bash
# Clone the repo
git clone https://github.com/seanchatmangpt/lsp-max.git
cd lsp-max

# Create a new branch
git checkout -b feature/my-feature

# Set up the workspace
just setup

# Verify everything builds
just check
```

### Making Changes

1. **Pick an issue or RFC** — Check the issue tracker or `docs/rfcs/` for work that needs doing.
2. **Write code** — Follow the style guide below.
3. **Add tests** — Every feature needs at least one test. Place tests in `#[cfg(test)]` modules within the same file.
4. **Format and lint** — Run `just fmt` and `just clippy` before committing.
5. **Commit** — Write a descriptive commit message (conventional commits, see examples below).

### Conventional Commits

Commit messages follow the format `type(scope): description`:

- **type:** `feat`, `fix`, `refactor`, `docs`, `test`, `perf`, `chore`
- **scope:** the crate or module affected (e.g., `compositor`, `runtime`, `gate`)
- **description:** one-line summary (imperative mood, no period)

Examples:

```
feat(compositor): add tier-stratified routing for DiagnosticsOnly servers
fix(runtime): prevent ConformanceVector invariant violation on unknown→admitted transition
docs(architecture): unify receipt chain narrative across law-state documentation
test(gate): add adversarial test for Oracle class A11 (Unknown Collapse)
perf(scan): optimize regex matching via multi-pattern automaton
```

### Running Checks Before Commit

```bash
# Format and lint
just fmt
just clippy

# Run all tests
just test

# Build the release binary
just build-release

# Full pre-commit suite
just pre-commit
```

If any check fails, fix the issues and re-run before pushing.

## Style Guide

### Naming

- **Types:** `PascalCase` (e.g., `ConformanceVector`, `ChildServer`)
- **Functions:** `snake_case` (e.g., `scan_uri`, `from_flush_contribution`)
- **Constants:** `UPPER_SNAKE_CASE` (e.g., `MAX_CONCURRENT_SERVERS`)
- **Modules:** `snake_case` (e.g., `flush_coordinator`, `rule_pack_server`)
- **Private items:** prefix with `_` if intentionally unused (e.g., `_phantom`)

### Comments

- **Prefer self-documenting code** — choose clear names and structure over comments.
- **Doc comments** (`///`) on public APIs with examples. Every public function, trait, and struct should have a doc comment.
- **Inline comments** (two spaces before `//`) only for non-obvious logic: hidden constraints, subtle invariants, workarounds for specific bugs.
- **No comments explaining WHAT the code does** — if you need to explain what, the code needs renaming, not commenting.

Example:

```rust
/// Build evidence for a child contribution tied to a specific code symbol.
/// `symbol_object_id` becomes the Phase-A join key `moniker:{scheme}:{identifier}`.
pub fn for_symbol(
    server_id: impl Into<String>,
    receipt: CryptographicReceipt,
    scheme: &str,
    identifier: &str,
    has_andon_contribution: bool,
) -> Self {
    Self {
        server_id: server_id.into(),
        receipt,
        symbol_object_id: Some(moniker_object_id(scheme, identifier)),
        has_andon_contribution,
    }
}
```

### Edition and MSRV

- **Edition:** 2021
- **MSRV (Minimum Supported Rust Version):** 1.70
- **Unsafe code:** Forbidden (`#![forbid(unsafe_code)]`) except in 3 justified blocks (see `SAFETY.md`). If you need unsafe, document the proof and get review.

### No External Dependencies (for Core Algorithms)

The `lsp-max-logic` and `lsp-max-runtime` crates avoid external dependencies to ensure:

- Zero-cost abstractions (no runtime overhead)
- Side-channel resilience (no hidden dynamic behavior)
- Auditability (all code is reviewable)

For other crates, dependencies are OK if justified. Document why in a comment at the `use` statement.

## Testing

### Unit Tests

Place tests in the same module as the code under test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conformance_vector_invariant_is_enforced() {
        let mut cv = ConformanceVector::new();
        cv.admitted.insert("feature1".to_string());
        cv.refused.insert("feature1".to_string());  // Violates invariant
        assert!(cv.validate().is_err());  // Must reject
    }
}
```

### Integration Tests

Create files in `tests/` for end-to-end scenarios:

```bash
tests/
├── integration_tests.rs      # Main integration test file
├── fixtures/
│   ├── minimal.toml
│   ├── complex_workspace/
│   └── ...
└── data/
    └── ...
```

### Property-Based Testing

Use `proptest` for randomized testing of invariants:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn receipt_chain_never_cycles(receipts in prop::collection::vec(any::<Receipt>(), 0..1000)) {
        for i in 1..receipts.len() {
            assert_ne!(receipts[i].prev_hash, receipts[i].consequence_hash);
        }
    }
}
```

### Running Specific Tests

```bash
# Run tests for a crate
cargo test -p lsp-max-compositor

# Run a specific test
cargo test -p lsp-max-runtime conformance_vector_invariant_is_enforced

# Run with output
cargo test -p lsp-max -- --nocapture
```

## Documentation

### Public APIs

Every public function, type, and trait must have a doc comment:

```rust
/// Build a compositor-signed chain link for a child's per-flush contribution.
///
/// The compositor signs on the child's behalf using its own `Keystore` because
/// the child has not yet published its own receipt file (that path is OPEN).
/// `admitted_count` and `has_andon` are folded into the `consequence_hash` via
/// BLAKE3 so the link is attributable to this specific flush, not a generic stub.
///
/// # Arguments
///
/// * `server_id` - Identifier of the child server emitting diagnostics
/// * `admitted_count` - Number of diagnostics that were admitted (not refused)
/// * `has_andon` - Whether an ANDON (refusal) diagnostic was emitted
/// * `sequence` - Monotonically increasing sequence number
/// * `prev_hash` - BLAKE3 digest of the prior receipt
/// * `keystore` - Keystore to sign this receipt
///
/// # Example
///
/// ```ignore
/// let receipt = ChildEvidence::from_flush_contribution(
///     "diagnostics-only-lsp", 5, false, 1, prev, &keystore
/// );
/// assert_eq!(receipt.server_id, "diagnostics-only-lsp");
/// ```
pub fn from_flush_contribution(
    // ...
) -> Self {
    // ...
}
```

### Architecture Docs

Major architectural changes require an RFC (see `docs/rfcs/README.md`). Before implementing:

1. Write the RFC in the standard format.
2. Get feedback via PR review.
3. Once approved, implement the RFC and link to it in your PR.

### Changelog

Update `CHANGELOG.md` with user-facing changes. Use CalVer version scheme (YY.M.D):

```markdown
## [26.7.1] - 2026-07-01

### Added
- Tier-stratified routing for DiagnosticsOnly servers (RFC 0004)
- `max/flushDiagnostics` RPC for explicit buffer flush

### Fixed
- ConformanceVector invariant violation on unknown→admitted transition

### Changed
- Deprecated: `lsp-max.toml` key `priority: "secondary"` → use `priority: "full"` + `primary_extensions` / `secondary_extensions` split
```

## Code Review

- **Be respectful.** We're all learning.
- **Focus on code, not person.** "This variable name is unclear" not "you chose a bad name."
- **Link to docs.** If suggesting a change, reference the style guide or architecture doc.
- **Be responsive.** Try to review PRs within 48 hours.
- **Approve only clean PRs.** If tests fail, linter complains, or docs are missing, request changes.

## Reporting Issues

If you find a bug or have a feature request:

1. **Check existing issues** to avoid duplicates.
2. **Title:** One-line description (e.g., "ConformanceVector invariant violated when unknown→admitted").
3. **Describe the problem:** What did you do? What did you expect? What happened instead?
4. **Minimal reproduction:** If possible, provide a small example that triggers the issue.
5. **Environment:** Rust version, OS, etc.

Example:

```
Title: ConformanceVector invariant violated on unknown→admitted transition

Description:
When transitioning a feature from unknown to admitted, the invariant
`admitted ∩ unknown = ∅` is violated.

Steps to reproduce:
1. Create a ConformanceVector with feature X in unknown
2. Transition X to admitted
3. Check that X is still in unknown (should not be)

Expected: X is removed from unknown
Actual: X remains in unknown

Environment: Rust 1.75, lsp-max v26.7.1
```

## Merging to Main

Merges to `main` require:

- ✅ All CI checks pass (build, tests, linter)
- ✅ At least one review approval
- ✅ Changelog entry (if user-facing)
- ✅ Conventional commit message
- ✅ No merge conflicts

## Release Process

Releases follow CalVer (YY.M.D):

1. **Create a release branch:** `git checkout -b release/26.7.1`
2. **Update version** in `Cargo.toml` and `CHANGELOG.md`
3. **Run full test suite:** `just test`
4. **Tag the commit:** `git tag v26.7.1`
5. **Push to main:** `git push origin release/26.7.1 && git push origin v26.7.1`
6. **Publish to crates.io** (maintainers only): `cargo publish --all`

## Getting Help

- **Questions:** Open a discussion (not an issue) on GitHub.
- **Design feedback:** Write an RFC and tag maintainers.
- **Build issues:** Run `just doctor` and attach the output to your issue.
- **Community:** Join the Rust Language Server Protocol community on Zulip (see `CONTRIBUTING.md` links).

## Further Reading

- **Code of Conduct:** `CODE_OF_CONDUCT.md`
- **Architecture:** `docs/book/01-architecture.md`
- **Design RFCs:** `docs/rfcs/README.md`
- **Getting Started:** `docs/book/03-getting-started.md`
