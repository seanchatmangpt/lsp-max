# ADR-0004: Anti-LLM-Cheat Diagnostic Canary

**Status:** ACCEPTED

## Context

Law enforcement in lsp-max relies on receipt chains and ConformanceVector invariants. However, if a developer (or an LLM in autonomous mode) accidentally reintroduces plain `tower-lsp`, uses fake receipt claims, writes victory language ("done", "solved", "guaranteed") in code comments, or violates other lsp-max laws, these violations may not surface until post-deployment.

The anti-llm-cheat-lsp example is an LSP server that runs during CI (via `just test-pre-publish` and the ANDON gate) and detects violations by examining the compiled codebase. It is a **compile-time canary** — a negative control that proves the system can detect and refuse illegal changes.

Early detection reduces the cost of law violations from post-deployment (very expensive) to compile time (cheap and immediate).

## Decision

Implement a dedicated example LSP server (`examples/anti-llm-cheat-lsp`) that performs the following checks at compile time (via AST analysis and diagnostics emission):

1. **Tower-lsp re-introduction**: Detects any use of `tower_lsp::` or `tower-lsp` imports outside of explicit negative-control test fixtures.
2. **Victory language**: Scans code and comments for words like "done", "all clean", "fully solved", "guaranteed", etc. Maps to the `ANTI-LLM-VICTORY-*` diagnostic family.
3. **Fake receipt artifacts**: Detects claims of "admission" without corresponding receipt artifacts in the codebase.
4. **Fake routes**: Identifies LSP method handlers that claim capabilities but have no implementation.
5. **Version law violations**: Checks that workspace version follows CalVer (YY.M.D), not SemVer.

The canary runs as part of `just dx-verify` and blocks the gate (ANDON) if violations are found. Code cannot be merged until the canary clears.

## Consequences

**Positive:**
- Early detection: law violations are caught at compile time, not in production.
- Automation: CI enforces the rules without manual code review.
- Audit trail: each diagnostic emission logs the violation, boundary, and suggested fix.
- Determinism: the same codebase always produces the same diagnostics; no false negatives.

**Negative:**
- False positives: the canary may flag legitimate uses of certain words (e.g., "done" in domain-specific vocabulary).
- Maintenance burden: the canary itself is code that must be tested and updated as laws evolve.
- Complexity: developers must understand the canary's rules to avoid violations.

**Neutral:**
- The canary is a workspace example, not part of the shipped lsp-max library.
- External projects that depend on lsp-max are not subject to the canary's checks; they enforce their own laws.

## Alternatives Considered

1. **Manual code review**: Relies on human vigilance; vulnerable to oversights.
2. **Linting as a separate tool**: Decouples checks from CI; easier to ignore.
3. **Type-system enforcement**: Would require non-standard Rust features (e.g., linear types); overkill.
4. **Runtime checks**: Defers violations to execution time; too late.

The diagnostic canary was chosen because it:
- Integrates into the LSP protocol itself (diagnostics are native LSP output).
- Runs at compile time, not runtime (fast, deterministic).
- Provides clear, actionable error messages to developers.
- Is self-hosting: the canary is an lsp-max LSP that can be tested against itself.

## Reference

- **Example crate**: `examples/anti-llm-cheat-lsp`
- **Diagnostic families**:
  - `ANTI-LLM-CHEAT-*`: illegal tower-lsp re-introduction
  - `ANTI-LLM-VICTORY-*`: victory language in code
  - `ANTI-LLM-FAKE-*`: fake receipt claims or routes
  - `ANTI-LLM-VERSION-*`: CalVer violations
- **CI integration**: `.github/workflows/` (runs as part of test-pre-publish)
- **Gate integration**: ANDON gate in `.claude/settings.json` blocks Bash until canary clears
