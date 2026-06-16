# ADR-0005: CalVer Versioning Over SemVer

## Status

ACCEPTED

## Context

lsp-max is versioned as `26.6.9`, not `2.0.0` (SemVer). The version format is **CalVer** (Calendar Versioning): `YY.M.D` where `YY` = last two digits of year, `M` = month (1–12), `D` = day (1–31).

As of 2025-06-14, the version is `26.6.14` (2026, June, 14th). This is the versioning standard for projects with **time-sensitive regulatory, contractual, or auditable requirements**:

- **Regulatory Software**: Medical devices, nuclear systems, financial trading platforms.
- **Temporal Audits**: "Which version was deployed on 2025-06-10?" is a direct query.
- **Legal Compliance**: Version identities with dates for compliance reports and release notes.

Early lsp-max design used SemVer (`0.1.0`, `0.2.0`, …, `1.0.0`). Tradeoffs emerged:

1. **SemVer encodes semantics** ("2.0.0 is breaking, 2.1.0 adds features, 2.1.1 fixes bugs"). This requires discipline: contributors must agree on what constitutes "breaking" for a law-state runtime.

2. **Time is implicit in SemVer**. Version `2.3.0` tells you nothing about when it was released; you must look up the git tag or release notes. For audit trails and process-mining, implicit time is fragile.

3. **Conformance Changes Are "Breaking"** in lsp-max. If a new law is introduced (e.g., "no legacy type crates"), the set of *valid* code shrinks; that's a breaking change. SemVer's MAJOR bump suggests external API instability, which is misleading. The API is stable; the *compliance bar* changed.

4. **Distributed Deployments**. lsp-max runs on both developer machines and CI systems, and may be embedded in sibling projects. Knowing "version X was deployed on 2025-05-20" (CalVer) is immediately actionable; "version X" (SemVer) requires lookups.

## Decision

Adopt **CalVer (YY.M.D)** as the sole versioning scheme:

- **Workspace version in `Cargo.toml`**: `version = "26.6.14"` (updated daily on active development, or per release).
- **Version Invariant**: Versions increase monotonically; `26.6.14` < `26.6.15` < `26.7.1`.
- **Release Cadence**: Releases are tagged per calendar day (or per significant event). Multiple releases per day are suffixed: `26.6.14-a`, `26.6.14-b`, etc.
- **Diagnostic Family**: Violations of version-law invariants are emitted as `ANTI-LLM-VERSION-*` diagnostics (e.g., `ANTI-LLM-VERSION-REGRESS` if a version number decreases).

## Consequences

### Benefits
- **Temporal Clarity**: Version numbers are self-documenting dates; no need to look up when a release happened.
- **Audit Trail**: For compliance and dispute resolution, "which code was running on 2025-05-20?" is answerable directly from version numbers across the stack.
- **Law-State Clarity**: Conformance changes (tightening compliance bars) are not conflated with API breakage. SemVer MAJOR bumps are reserved for true API incompatibility, which lsp-max aims to minimize.
- **CI Integration**: Release gates can query "deploy the version from 2025-06-10"; no ambiguity between release candidates.
- **Process Mining**: Temporal ordering of versions is guaranteed; downstream engines can assume monotonicity without checking.

### Tradeoffs
- **Spec Unfamiliarity**: CalVer is less common than SemVer in Rust; many contributors expect major.minor.patch. Requires documentation.
- **Epoch Exhaustion**: The 2-digit year (YY) overflows at 2099. Not a concern in practice (project lifespan is presumably <75 years), but requires a migration plan if the project outlives 2099.
- **Backward Compatibility Ambiguity**: SemVer MAJOR bumps make breaking changes explicit. CalVer has no such signal. A release `26.7.1` might introduce breaking changes (tightened laws) or be fully compatible. Must document in release notes.
- **Tooling Mismatch**: Some package registries or CI tools assume SemVer. crates.io accepts CalVer, but tooling around dependency resolution may be less mature.

### Mitigations
- **Release Notes Clarity**: Every release includes a section: "Conformance Changes" (new laws, stricter enforcement) and "API Changes" (deprecated methods, signature changes).
- **Version Regex in Tests**: `anti-llm-cheat-lsp` validates that all versions in the workspace follow `\d{2}\.\d{1,2}\.\d{1,2}(-[a-z])?` and are monotonically increasing.
- **Documentation**: CLAUDE.md, AGENTS.md, and Justfile include version expectations. New contributors are directed to ADR-0005.

## Alternatives

### A1: Semantic Versioning (SemVer)
Use `MAJOR.MINOR.PATCH` (e.g., `2.3.0`). MAJOR bumps for breaking changes, MINOR for features, PATCH for bug fixes.

**Rejected**: Conformance tightening (stricter laws) is neither API-breaking nor a feature; it doesn't fit SemVer semantics. Also, temporal information is lost; releases from 2025-05-20 are indistinguishable from those from 2024-03-10 by version number alone.

### A2: SemVer + Dates in Release Notes
Keep SemVer (`2.3.0`); embed date metadata in release notes or git tags.

**Rejected**: Dates are not in the version string itself; CI systems and logs see only `2.3.0`, not the date. Requires dual-lookup (version → tag → release notes). Also, doesn't solve the "conformance change ≠ breaking API" problem.

### A3: Hybrid (SemVer with Date Metadata)
Version format: `MAJOR.MINOR.PATCH-YY.M.D` (e.g., `2.3.0-26.6.14`).

**Rejected**: Verbose; confusing for users who expect SemVer-only. Metadata after the dash is conventionally for pre-release (alpha, beta) or build, not dates. Registries might parse incorrectly.

### A4: Pure Date (YYYY-MM-DD or YYYYMMDD)
Version as a full date (e.g., `20260614` or `2026-06-14`).

**Rejected**: crates.io version format conventions prefer dot notation. Full date is verbose in release notes and commit messages. CalVer (YY.M.D) offers the same clarity with better compactness.

## References

- `Cargo.toml` (root) — `version = "26.6.14"`.
- `lsp-max-protocol/Cargo.toml`, `lsp-max-runtime/Cargo.toml`, etc. — All workspace members inherit root version via `version = { workspace = true }`.
- `examples/anti-llm-cheat-lsp/src/version_check.rs` — Regex validation and monotonicity checks.
- `examples/anti-llm-cheat-lsp/tests/dogfood_gc005_version_invariant.rs` — Test: version numbers are valid CalVer and monotonic.
- CLAUDE.md — Versioning section.
- AGENTS.md — Version-law enforcement ("Scope of Enforcement: Versioning").
- Justfile — `version-bump` recipe to increment version (updates all `Cargo.toml` files and git tags).
