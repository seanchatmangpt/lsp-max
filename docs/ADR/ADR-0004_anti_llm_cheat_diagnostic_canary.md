# ADR-0004: Anti-LLM-Cheat Diagnostic Canary

## Status

ACCEPTED

## Context

lsp-max enforces architectural laws via CLAUDE.md and AGENTS.md:
- Never use plain `tower_lsp` in production code or tests.
- No victory language ("done", "solved", "all clean") in code comments, commit messages, or diagnostics.
- ConformanceVector's three-valued logic must never collapse (Unknown→Admitted via silent upgrade).
- Receipt claims in diagnostics must be backed by receipt artifacts; no fake receipts.
- No intermediary type crates (wasm4pm_types, ocel_core) in the type authority chain.

These laws are **aspirational**. Code review and human inspection can enforce them, but humans are fallible, especially when working at scale or with LLM-driven commits. An LLM (including future Claude versions) might unintentionally violate a law while implementing a feature; or a contributor might claim a law is obeyed when it isn't.

Without automated enforcement, laws become "suggestions." Over time, violations compound: a single `use tower_lsp::*` in a test becomes a pattern; victory language spreads via copy-paste; receipt forgery goes undetected.

**The Solution: Anti-LLM-Cheat as a Compile-Time Canary**

If every code commit must pass a linter that detects law violations *before merge*, then violations cannot be *undetected* (only *detected and consciously accepted*). This inverts the burden: violations must be explicit exceptions, not silent accumulation.

## Decision

Implement `examples/anti-llm-cheat-lsp` — an LSP server that **runs as a linter** and emits MaxDiagnostic entries for every law violation found in the codebase:

1. **Stateless Scan**: On every `lsp-max-cli gate check` run, scan the entire workspace for violations (not just changed files).
2. **Violation Detection**:
   - Regex: `use tower_lsp` (or `tower-lsp` in Cargo.toml imports) → `ANTI-LLM-TOWER-LSP-USAGE`
   - Regex: Victory language (`\b(done|solved|all clean|fully admitted|guaranteed|fixed)\b` in code/comments/commits) → `ANTI-LLM-VICTORY-LANGUAGE`
   - AST: UnknownConformance that collapsed to Admitted without explicit receipt → `ANTI-LLM-UNKNOWN-COLLAPSE`
   - File: Diagnostic claiming receipt path that does not exist in `.lsp-max-receipts/` → `ANTI-LLM-FAKE-RECEIPT`
   - Cargo.toml: Type crates (`wasm4pm_types`, `ocel_core`, `*legacy*`) in `[dependencies]` or `[patch.crates-io]` → `ANTI-LLM-FORBIDDEN-TYPE-CRATE`
   - Manifest: References to `tower-lsp` or `tower_lsp` outside explicit negative-control fixtures → `ANTI-LLM-TOWER-LSP-IMPORT`

3. **Diagnostic Output**: Each violation is a MaxDiagnostic with:
   - `source: "anti-llm-cheat"`
   - `severity: Error` (blocks gate)
   - `rule: String` (e.g., "ANTI-LLM-TOWER-LSP-USAGE")
   - `message: String` (specific violation + fix suggestion)
   - `receipt: Option<DiagnosticReceipt>` (null unless the violation itself is a fake receipt claim)

4. **Gate Integration**: `lsp-max-cli gate check` runs anti-llm-cheat before final clearance. If any diagnostics are emitted, gate status is BLOCKED.

5. **Negative Control**: anti-llm-cheat itself is a workspace member (not an external tool); it compiles under the same rules it enforces. Any law violation in anti-llm-cheat is doubly embarrassing.

## Consequences

### Benefits
- **Automated Enforcement**: No law violation can be undetected; they are emitted as diagnostics on every build.
- **LLM-Resistant**: Even if an LLM-driven commit violates a law, the canary catches it before merge.
- **Negative Control**: By living in the workspace, anti-llm-cheat is dogfooded; its own violations would block its tests.
- **Clear Audit Trail**: Every violation is a timestamped diagnostic with rule, location, and suggested fix.
- **Incentive Alignment**: Contributors cannot claim "all laws are obeyed" unless anti-llm-cheat agrees; claims are falsifiable.

### Tradeoffs
- **Maintenance Burden**: New laws require new regex/AST patterns in anti-llm-cheat; lag between law definition and enforcement.
- **False Positives**: Regex patterns are fragile. A comment containing `"tower_lsp"` in a docstring or a variable named `done_count` would be falsely flagged. Requires careful exception lists.
- **Performance**: Scanning the entire workspace on every gate check is O(n); large projects may see latency. Mitigated by caching (gate check on unchanged files uses prior receipt digest).
- **Visibility**: Violations are emitted as LSP diagnostics (not build errors). A contributor might ignore them. Mitigated by PreToolUse hook that blocks Bash if gate is BLOCKED.

## Alternatives

### A1: Rely on Code Review
Human reviewers catch violations during PR review; violations are discussed and corrected before merge.

**Rejected**: Scale doesn't work. With 10+ contributors or LLM-assisted commits, violations slip through. Review is async; lag between violation and detection is high.

### A2: Post-Merge Audit via CI Script
After merge, a CI script scans for violations and opens a ticket.

**Rejected**: Violation is already in main; damage is done. Post-detection doesn't prevent spread; exceptions accumulate.

### A3: Runtime Enforcement Only (No Compile-Time Check)
Laws are checked at server startup or during initialization; violations cause runtime errors.

**Rejected**: Too late. A violation in code that doesn't run until a rare code path is activated might not be caught for months. Compile-time is earlier feedback.

### A4: External Linter (Not a Workspace Member)
Build an anti-llm-cheat tool outside the workspace; run it as part of CI.

**Rejected**: Decoupling loses the "canary" property. External tool might have its own violations; no self-dogfood. Also, tight coupling to specific lsp-max laws means the external tool must be updated every time a law changes.

## References

- `examples/anti-llm-cheat-lsp/src/lib.rs` — Core detection logic (regex, AST, file checks).
- `examples/anti-llm-cheat-lsp/tests/dogfood_gc001_tower_lsp_usage.rs` — Test: tower-lsp detection.
- `examples/anti-llm-cheat-lsp/tests/dogfood_gc002_victory_language.rs` — Test: victory language detection.
- `lsp-max-cli/src/gate.rs` — `gate check` invokes anti-llm-cheat.
- `.claude/settings.json` — PreToolUse hook: `lsp-max-cli gate check` blocks Bash if ANDON is set.
- AGENTS.md — Anti-LLM-Cheat section ("Compile-Time Law Enforcement").
- CLAUDE.md — Law definitions ("Never reference plain tower-lsp", "No victory language").
