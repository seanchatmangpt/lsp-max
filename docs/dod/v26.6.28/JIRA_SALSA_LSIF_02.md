# JIRA: SALSA_LSIF_02 - Step 2: Salsa LSIF Document Index

**Status:** ADMITTED
**Epic:** v26.6.28 — Salsa + LSIF Semantic Memory Admission

## Description
Implement the Salsa LSIF Document Index to ensure that incremental indexing is Update-safe and does not retain non-Update-safe objects across computations.

## Requirements
- `index_document` must be tracked.
- Input to the index must be source text, path, or digest.
- Output from the index must be `LsifFileResult` or Update-safe rows, counts, and digests.

## Acceptance Criteria
- Cached results must be reused when the source is unchanged.
- The LSIF document index must recompute when the source changes.
- `index_document` must not store parser handles, file handles, writer handles, or Oxigraph handles.

## Invariants

**Required Law:** `LSIF_DOCUMENT_INDEX_IS_INCREMENTAL_AND_UPDATE_SAFE`

- **TRUE:** `index_document` is tracked. Input is source text/path/digest. Output is `LsifFileResult` or Update-safe rows/counts/digests.
- **FALSE:** `index_document` stores parser handles, file handles, writer handles, or Oxigraph handles.
- **COUNTERFACTUAL:**
  - change source → LSIF document index recomputes
  - unchanged source → cached result reused
- **WITNESS:** `cargo test -p lsp-max-lsif`
- **REPAIR:** Move non-Update objects into helper functions and return derived facts only.
