# Architecture Decision Records (ADR)

This directory contains Architecture Decision Records (ADRs) in MADR format that document major architectural decisions in lsp-max.

All ADRs in this directory have status **ACCEPTED** and reflect decisions that have been realized in the codebase.

## Index

- **[ADR-0001: Law Enforcement via Receipt Chains](ADR-0001_law_enforcement_via_receipt_chains.md)**
  - Why receipt ledgers with SHA256 chains, not just status bits
  - Auditability, forensic recovery, and replay safety
  
- **[ADR-0002: Conformance Vector Three-Valued Logic](ADR-0002_conformance_vector_three_valued_logic.md)**
  - Why admitted/refused/unknown must stay disjoint, not collapse to binary
  - Epistemic clarity and law-state integrity

- **[ADR-0003: Composition Over tower-lsp Fork](ADR-0003_composition_over_tower_lsp_fork.md)**
  - Why lsp-max forked tower-lsp instead of extending it
  - Five-layer isolation model and law-state architecture

- **[ADR-0004: Anti-LLM-Cheat Diagnostic Canary](ADR-0004_anti_llm_cheat_diagnostic_canary.md)**
  - Why `examples/anti-llm-cheat-lsp` detects law violations at compile time
  - Early detection, automation, and audit trails

- **[ADR-0005: CalVer Versioning Over SemVer](ADR-0005_calver_versioning_over_semver.md)**
  - Why version scheme is `26.6.9` (YY.M.D), not semantic versioning
  - Build traceability, temporal compliance, and LSP stability alignment

## Format

Each ADR follows the MADR template with sections:

1. **Status**: ACCEPTED, PROPOSED, or SUPERSEDED (all current ADRs are ACCEPTED)
2. **Context**: The problem or decision point
3. **Decision**: The architectural choice made
4. **Consequences**: Positive, negative, and neutral outcomes
5. **Alternatives Considered**: Other options evaluated and why they were rejected
6. **Reference**: Related crates, files, diagnostic families, and tests

## Related Documentation

- **AGENTS.md**: Project constitution; laws enforced by tooling
- **CLAUDE.md**: Project guidance for Claude Code; architecture overview
- **CONTRIBUTING.md**: Contribution guidelines aligned with ADR decisions

## Enforcement

ADRs are enforced via:

- **Compile-time checks**: `examples/anti-llm-cheat-lsp` detects violations (tower-lsp re-introduction, victory language, version scheme violations)
- **CI gates**: `just test-pre-publish` and `just dx-verify` block merges if violations are detected
- **ANDON gate**: `.claude/settings.json` PreToolUse hook runs `lsp-max-cli gate check` before shell commands
