# [LSPMAX-04] Stale LSIF Index ANDON

**Status:** ADMITTED
**Epic:** lsp-max v26.6.28 — Salsa + LSIF Semantic Memory Admission

## Description
This ticket covers Step 4 of the Definition of Done: enforcing `STALE_LSIF_INDEX = STOP` ANDON state. Stale LSIF is worse than missing LSIF because it gives false structural confidence. The system must actively push an ANDON event if the LSIF index falls out of sync with the canonical source.

## Governing Equation
```text
R_B ⊢ A = μ(O*_B)
```

## Requirements
- Stale LSIF must halt admission (`STALE_LSIF_INDEX_IS_STOP`)
- Virtual documents or diagnostics are insufficient on their own; an ANDON push must occur (`STALE_LSIF_REQUIRES_ANDON_PUSH`)
- Semantic memory cannot be used without a valid, non-stale receipt (`SEMANTIC_MEMORY_REQUIRES_RECEIPT`)

## Acceptance Criteria (Invariant Rules)

### Invariant 1: STALE_LSIF_INDEX_IS_STOP
- **TRUE**: `source_digest == receipt.source_digest` AND `lsif_digest == receipt.lsif_digest` AND LSIF file exists AND `receipt.status == ADMITTED`.
- **FALSE**: Source changed after receipt, digest mismatch, missing file, or `status != ADMITTED`.
- **COUNTERFACTUAL**: Modify indexed source after receipt → `STALE_LSIF_INDEX` fires → `severity = STOP` → `admission_allowed = false`.
- **WITNESS**: LSIF receipt, artifact digest, current source digest, gate state, ANDON event.
- **REPAIR**: Rerun LSIF indexer, regenerate receipt, rerun stale-index check.
- **DIAGNOSTIC**: `LSPMAX-LSIF-STALE-INDEX` (Severity: STOP).

### Invariant 2: STALE_LSIF_REQUIRES_ANDON_PUSH
- **TRUE**: Stale LSIF produces diagnostic AND ANDON event AND `admission_allowed = false` AND repair command exposed.
- **FALSE**: Stale LSIF appears only in virtual doc or log.
- **COUNTERFACTUAL**: Disable ANDON push while stale LSIF diagnostic exists → `LSPMAX-ANDON-PUSH-MISSING` fires.
- **EVENT**: `lspMax/andonRaised`.
- **REPAIR**: `cargo run -p lsp-max-lsif -- --root <scope> --out receipts/v26.6.28-lsif.lsif`.

### Invariant 3: SEMANTIC_MEMORY_REQUIRES_RECEIPT
- **TRUE**: LSIF artifact has admitted receipt, digests match, stale check passes.
- **FALSE**: System (agent, LSP, Oxigraph) uses LSIF artifact as project memory without an admitted receipt.
- **COUNTERFACTUAL**: Delete receipt but leave LSIF file → semantic memory refused → `LSPMAX-SEMANTIC-MEMORY-WITHOUT-RECEIPT` fires.
- **WITNESS**: LSIF artifact, LSIF receipt, invariant result, gate context.
- **REPAIR**: Regenerate receipt, rerun stale check.
