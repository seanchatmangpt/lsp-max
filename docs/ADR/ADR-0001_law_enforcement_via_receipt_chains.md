# ADR-0001: Law Enforcement via Receipt Chains

**Status:** ACCEPTED

## Context

The lsp-max project is a law-state runtime projected through LSP — it enforces a set of architectural laws and invariants that must hold throughout the lifetime of the system. Early designs considered using simple boolean status bits (e.g., `is_admitted: bool`) to track whether a decision or action conformed to law.

However, simple status bits have critical limitations:
- No auditability: a change from `false` to `true` leaves no trace of *when*, *how*, or *why* the transition occurred.
- No causal chain: downstream systems cannot verify that prior steps actually executed in lawful order.
- No forensic recovery: if a law violation is discovered post-hoc, there is no ledger to trace the violation's root cause.
- Vulnerability to replay: an adversary (or buggy code) could mutate a status bit directly without executing the lawful transition logic.

The anti-llm-cheat-lsp diagnostic canary compiles code and checks for violations of lsp-max laws. A law violation detected *after* a system has been deployed is far more costly than one caught *during* development. Thus, auditability at the point of decision is non-negotiable.

## Decision

Enforce law compliance via **receipt chains**: immutable ledger entries that form a cryptographic hash chain (SHA256 boundaries and digests). Each receipt captures:

1. **Boundary**: the point in the execution timeline where a decision was made.
2. **Digest**: a SHA256 hash that commits to the decision, its inputs, and the law-axis that governs it.
3. **Predecessor**: the digest of the prior receipt in the chain (or null for the initial receipt).

A transition from one state to another is lawful only if a receipt with a valid `ConformanceVector` (admitted/refused/unknown) and a valid cryptographic path exists. Status bits are derived *from* receipts, never written directly.

Tools and gates may refuse execution unless a receipt chain supports the claim. The diagnostic canary can verify that code attempting to claim "admission" actually produced a receipt artifact with a valid chain.

## Consequences

**Positive:**
- Auditability: every law-bearing decision leaves an immutable trace.
- Forensic recovery: violations can be traced backward through the chain to the root cause.
- Testability: unit tests and CI can verify that receipt chains are present and valid before accepting claims.
- Replay safety: a receipt cannot be forged or reused without recomputing the entire chain.

**Negative:**
- Complexity: developers must thread receipt generation through every decision point.
- Storage: receipts consume disk and memory (mitigated by archive and compaction policies).
- Verification overhead: computing SHA256 chains on every state transition incurs CPU cost.

**Neutral:**
- The receipt format is orthogonal to LSP itself; it is internal to the runtime.
- Existing LSP clients (editors) are unaffected; they see diagnostics and intents, not receipts.

## Alternatives Considered

1. **Status bits only**: Simplest but loses auditability and forensic power.
2. **Event log (unverified)**: Provides auditability but no cryptographic protection against tampering.
3. **Distributed ledger (blockchain)**: Overcomplicated for a single LSP process; overkill for law enforcement.
4. **Assertion-based (trusted compilation)**: Relies on compiler trust; vulnerable to post-compile tampering.

Receipt chains were chosen because they provide auditability, forensic recovery, and replay safety with minimal external dependencies and localized verification (no distributed consensus needed).

## Reference

- **Diagnostic family**: `LAW-*` (detected by anti-llm-cheat-lsp)
- **Key crate**: `lsp-max-protocol` (receipt types), `lsp-max-runtime` (receipt generation and verification)
- **Key file**: `src/gate.rs` (gate check logic validates receipt chains)
