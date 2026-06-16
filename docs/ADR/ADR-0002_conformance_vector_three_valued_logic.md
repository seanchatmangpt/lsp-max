# ADR-0002: Conformance Vector Three-Valued Logic

**Status:** ACCEPTED

## Context

LSP 3.18 capability declarations and conformance analysis require expressing whether a feature is supported. Early designs used binary logic: `supported: bool`. However, a strict binary model collapses important epistemic distinctions:

- **Admitted**: The feature is implemented, tested, and has receipt artifacts proving compliance.
- **Refused**: The feature *cannot* be supported due to architectural law or external constraint; refusal is deliberate and documented.
- **Unknown**: The feature status is indeterminate; analysis is incomplete, or the question is inapplicable.

Binary logic forces "unknown" to collapse into one of the other two, destroying information and leading to false claims:
- Collapsing into "admitted" risks claiming compliance for untested features.
- Collapsing into "refused" hides incomplete analysis and blocks valid development paths.

The ConformanceVector type carries three independent axis sets: `admitted`, `refused`, and `unknown`. These sets are **disjoint by law** — a feature cannot simultaneously be admitted and refused, nor can admitted or refused collapse into unknown.

## Decision

Implement `ConformanceVector` with three mutually exclusive axis sets:

```rust
pub struct ConformanceVector {
    pub admitted: HashSet<String>,    // Features with receipt artifacts
    pub refused: HashSet<String>,     // Features blocked by law/constraint
    pub unknown: HashSet<String>,     // Features with indeterminate status
}
```

Invariants:
1. `admitted ∩ refused = ∅` (no feature is both admitted and refused)
2. `admitted ∩ unknown = ∅` (no feature is both admitted and unknown)
3. `refused ∩ unknown = ∅` (no feature is both refused and unknown)

Any operation that would violate these invariants fails with a diagnostic in the `CONFORMANCE-*` family. Transitions between states (e.g., from unknown to admitted) are valid only if accompanied by receipt artifacts.

## Consequences

**Positive:**
- Epistemic clarity: unknown is not conflated with admitted or refused.
- Bidirectional: unknown can transition to admitted *or* refused; the system is not locked into a false binary choice.
- Testability: conformance tests can verify all three states independently.
- Law enforcement: refusal axes are explicit and auditable; no silent collapses.

**Negative:**
- Complexity: three-valued logic is less intuitive than binary.
- Query cost: checking set membership is O(n) per query; mitigated by hashing and caching.
- API surface: downstream code must handle three cases instead of two.

**Neutral:**
- LSP 3.18 capability reporting remains unaffected; clients still see boolean `supported` fields.
- The three-valued logic is internal to lsp-max; translation to LSP boolean is explicit at the boundary.

## Alternatives Considered

1. **Binary logic**: Simple but loses epistemic distinction; forces false claims.
2. **Four-valued (true/false/unknown/error)**: Adds error state; redundant given receipt chains capture errors.
3. **Partial order (lattice)**: Over-engineered for law-state tracking.
4. **Fuzzy logic (probabilities)**: Incompatible with binary receipt chains; introduces spurious confidence.

Three-valued logic (admitted/refused/unknown) was chosen because it:
- Maps naturally to the law-state runtime's three possible outcomes.
- Preserves epistemic honesty (unknown stays unknown until proven otherwise).
- Integrates seamlessly with receipt chains (receipts prove admitted; refusal receipts prove refused).

## Reference

- **Type definition**: `lsp-max-protocol/src/conformance.rs`
- **Diagnostic family**: `CONFORMANCE-*` (set membership violations)
- **Testing**: `tests/test_conformance_vectors.rs`
- **Key crate**: `lsp-max-runtime` (enforces invariants at runtime)
