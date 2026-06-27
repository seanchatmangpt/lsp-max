# JIRA: SALSA_AST_01 - Step 1: Salsa AST Slice

**Status:** ADMITTED
**Epic:** v26.6.28 — Salsa + LSIF Semantic Memory Admission

## Description
Implement the Salsa vertical slice for the AST to ensure that Salsa does not own or store non-update-safe objects like `tree_sitter::Tree`. This enforces the tracking of only Update-safe facts in the hot-path incremental computation.

## Requirements
- `parse_document` must act as a helper-only function and must not be a tracked query.
- `ast_diagnostics` must be a tracked query.
- Tracked query outputs must be Update-safe.

## Acceptance Criteria
- Admitted outputs for tracked queries must be limited to: `Vec<SalsaDiag>`, `Vec<LsifRow>`, `LsifFileResult`, `SymbolFact`, `Digest`, `TruthTableRow`, `InvariantFact`.
- Refused outputs inside tracked queries must include: `tree_sitter::Tree`, `Parser`, file handles, LSP client handles, Oxigraph handles, Papaya guards, DashMap guards, open writers, raw runtime handles.

## Invariants

**Required Law:** `SALSA_DOES_NOT_OWN_TREE_SITTER_TREE`

- **TRUE:** `parse_document` is helper-only. `ast_diagnostics` is tracked. Tracked output is Update-safe.
- **FALSE:** `tree_sitter::Tree` stored inside tracked query output.
- **COUNTERFACTUAL:** Returning `tree_sitter::Tree` from a tracked query must result in a compile refusal and yield `LSPMAX-SALSA-NONUPDATE-TREE-REFUSED`.
- **WITNESS:** `cargo test -p lsp-max-ast`
- **REPAIR:** Return Update-safe facts: `SalsaDiag`, `SymbolFact`, `Digest`, `TruthTableRow`, `LsifFileResult`.

**Tracked Query Output Law Diagnostic:** `LSPMAX-TRACKED-OUTPUT-NONUPDATE-REFUSED`
