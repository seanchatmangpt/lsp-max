# [LSPMAX-03] LSIF Digest Receipt Admission

**Status:** ADMITTED
**Epic:** lsp-max v26.6.28 — Salsa + LSIF Semantic Memory Admission

## Description
This ticket covers Step 3 of the Definition of Done: the admission of the LSIF Digest Receipt. The LLM Disclaimer Gap must be closed by ensuring codebase structure is receiptable. No LSIF artifact can be considered project memory without a corresponding, mathematically binding digest receipt that ensures structure and stability.

## Governing Equation
```text
R_B ⊢ A = μ(O*_B)
```

## Requirements
- Generate LSIF digest receipt at `receipts/v26.6.28-lsif.receipt.json`
- Generate LSIF artifact at `receipts/v26.6.28-lsif.lsif`
- The receipt shape must include deterministic counts, source boundary, and source digest (canonical)
- `LSIF_RECEIPT_MUST_NOT_HASH_ITSELF`: The receipt generation must not include the `receipts/` directory or volatile generated outputs in its source boundary
- `SOURCE_DIGEST_IS_CANONICAL`: The source digest must be canonical and independent of filesystem traversal order, temporary files, etc.
- `LSIF_COUNTS_ARE_DETERMINISTIC`: Reference counts and vertex counts must be strictly defined.

## Acceptance Criteria (Invariant Rules)

### Invariant 1: LSIF_DIGEST_RECEIPT_ADMITTED
- **TRUE**: exit_code == 0, lsif_file_exists, blake3(lsif_file) == receipt.lsif_digest, blake3(canonical_source_boundary) == receipt.source_digest, counts are > 0, rules declared, receipt does not hash itself.
- **FALSE**: Missing receipt, digest mismatch, or missing counts.
- **WITNESS**: LSIF artifact, LSIF receipt, canonical source boundary.

### Invariant 2: LSIF_RECEIPT_MUST_NOT_HASH_ITSELF
- **TRUE**: `source_boundary` excludes `receipts/`, `target/`, `.git/`, `tmp/`, and generated outputs.
- **FALSE**: `source_digest` includes the receipt itself or LSIF artifact.
- **COUNTERFACTUAL**: Include `receipts/` in source boundary → `LSPMAX-LSIF-RECEIPT-SELF-REFERENCE` fires → `status = REFUSED`.
- **REPAIR**: Exclude generated outputs, sort file list, regenerate digest and receipt.

### Invariant 3: SOURCE_DIGEST_IS_CANONICAL
- **TRUE**: File list is sorted, paths are stable, excluded directories declared, bytes hashed in stable order.
- **FALSE**: Digest depends on filesystem traversal order, `receipts`, temp files, etc.
- **DIAGNOSTIC**: `LSPMAX-LSIF-SOURCE-BOUNDARY-UNSTABLE`.

### Invariant 4: LSIF_COUNTS_ARE_DETERMINISTIC
- **TRUE**: Counts extracted using mechanical rules (e.g., vertex_count = lines where `"type":"vertex"`).
- **FALSE**: Counts inferred informally, missing, or mismatched.
- **DIAGNOSTIC**: `LSPMAX-LSIF-COUNT-RULE-MISSING`, `LSPMAX-LSIF-COUNT-MISMATCH`, `LSPMAX-LSIF-REFERENCE-COUNT-NONDETERMINISTIC`.
