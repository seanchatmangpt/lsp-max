# RFC 0005: CalVer Versioning Over SemVer

**Status:** Accepted

## Context

Semantic versioning (SemVer) uses the scheme MAJOR.MINOR.PATCH to signal breaking changes, feature additions, and bug fixes. It is the default choice for Rust libraries and is well-understood by package managers (Cargo, npm, PyPI).

However, `lsp-max` is not a traditional library. It is:

- A law-state runtime with a fixed enforcement model (laws do not change between versions).
- A time-sensitive system where the current date is operationally relevant (compliance windows, temporal constraints, audit trails).
- A release-gated system where versions are tied to deployment windows, not feature additions.
- An LSP server with a stable protocol (LSP 3.18 is the target; the spec does not evolve within a single calendar period).

SemVer implies:

- 1.0.0 → 1.1.0 is a minor change; 2.0.0 is a breaking change.
- Version bumping is driven by feature scope, not time.
- The version number communicates nothing about when the code was released.

For a law-state runtime, these assumptions do not hold. A version number should communicate:

- When the code was deployed (build traceability for audit logs).
- What calendar period governed the deployment (temporal compliance windows).
- The stability of the runtime (all versions within the same calendar year have equivalent law-state machinery).

## Decision

Adopt **Calendar Versioning (CalVer)** with the scheme `YY.M.D`:

- **YY:** Two-digit year (e.g., 26 for 2026).
- **M:** Month without leading zero (e.g., 6 for June).
- **D:** Day without leading zero (e.g., 14 for the 14th).

Example: version `26.6.24` released on June 14, 2026.

All version bumps are tied to release deployments, not feature scope. Within a single calendar day, there is at most one release; if multiple releases are needed, append a pre-release identifier (e.g., `26.6.24-rc.1`, `26.6.24-rc.2`).

The workspace `Cargo.toml` enforces this scheme via CI checks. Any attempt to introduce SemVer-style versions (e.g., 1.0.0, 2.3.5) triggers a diagnostic and blocks the gate.

## Rationale

1. **Build traceability:** The version number instantly communicates the deployment date.

2. **Temporal compliance:** Audit logs can query versions by time window without additional metadata.

3. **Simplicity:** No decision needed about whether a change is MAJOR or MINOR; time decides.

4. **Alignment with LSP stability:** LSP 3.18 is the law; version numbers do not imply LSP spec changes (RFC 0001).

## Consequences

**Positive:**
- Build traceability: the version number instantly communicates the deployment date.
- Temporal compliance: audit logs can query versions by time window without additional metadata.
- Simplicity: no decision needed about whether a change is MAJOR or MINOR; time decides.
- Alignment with LSP stability: LSP 3.18 is the law; version numbers do not imply LSP spec changes.

**Negative:**
- Unfamiliarity: CalVer is less common in Rust; developers must learn the scheme.
- Sorting challenges: lexicographic sorting works for dates (26.6.24 < 26.6.15 < 26.7.1) but not for cross-year boundaries (26.12.31 > 27.1.1 lexicographically, but chronologically earlier).
- Package manager confusion: Cargo expects SemVer; CalVer versions may trigger warnings or downgrade behavior.

**Neutral:**
- Semantic meaning is sacrificed for temporal meaning; consumers who care about API stability must rely on other signals (e.g., LSP feature set, documented breaking changes).
- CalVer does not prevent parallel development or branching; it only structures released versions.

## Alternatives Considered

1. **Semantic versioning:** Standard but loses temporal traceability; version bumps are decoupled from deployments.
2. **Timestamp versioning** (e.g., `20260614`): Maximally precise but verbose and harder to read.
3. **Year-only** (e.g., `26`): Too coarse; multiple releases per year are common.
4. **Hybrid** (e.g., `26.6.24.PATCH`): Adds SemVer's PATCH component for same-day releases; introduces ambiguity.

CalVer (YY.M.D) was chosen because it provides daily granularity (sufficient for typical release cadences), is easy to read and remember, integrates cleanly with audit logs and compliance windows, and avoids the false precision of timestamp versioning.

## Reference

- **Enforced in:** `Cargo.toml` (workspace version field), `lsp-max-cli` (version reporting)
- **Validation:** CI checks that all crate versions match CalVer schema
- **Testing:** `tests/test_calver_enforcement.rs` validates version parsing and sorting
