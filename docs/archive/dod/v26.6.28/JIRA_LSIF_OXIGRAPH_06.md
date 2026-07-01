# [JIRA-06] Step 6: LSIF -> Oxigraph Snapshot Import

## Status
**ADMITTED**

## Definition of Done

This ticket governs the process of importing an admitted LSIF structure into Oxigraph. The LSIF is structure, and Oxigraph is meaning. Oxigraph cannot act as the raw code indexer; it requires an ADMITTED LSIF snapshot to function.

### 1. LSIF -> Oxigraph Snapshot Import

**Required Law:** `LSIF_IS_STRUCTURE_OXIGRAPH_IS_MEANING`

**Acceptance Criteria (TRUE):**
- LSIF provides code structure facts.
- Oxigraph imports those facts into a named graph (`urn:lsp-max:lsif:v26.6.28:<lsif_digest>`).
- Oxigraph joins code structure to semantic law.
- Oxigraph does not become the indexer.

**Rejection Criteria (FALSE):**
- Oxigraph replaces LSIF as the source-code indexer.
- LSIF snapshot is imported before the LSIF receipt is `ADMITTED`.

**Counterfactual Probe:**
- **Action:** Attempt LSIF -> Oxigraph import with a missing or stale LSIF receipt.
- **Expected Result:** `LSPMAX-LSIF-IMPORT-WITHOUT-RECEIPT` (Severity: `REFUSE`).

**Required RDF Fact Families:**
- `document`, `symbol`, `definition`, `reference`, `moniker`, `contains`, `refersTo`, `definedAt`

**Snapshot Admission Formula:**
```text
LSIF_OXIGRAPH_SNAPSHOT_ADMITTED =
  lsif_receipt.status == ADMITTED
  ∧ stale_lsif_index == false
  ∧ rdf_snapshot_digest exists
  ∧ named_graph_uri includes lsif_digest
  ∧ triple_count > 0
  ∧ Oxigraph import occurs outside hot path
  ∧ SPARQL smoke queries pass
```

**Witness Tests Required:**
- `lsif_import_requires_admitted_receipt`
- `lsif_snapshot_import_creates_named_graph`
- `lsif_snapshot_import_counts_triples`
- `named_graph_uri_contains_lsif_digest`
- Falsification tests (`lsif_import_before_receipt_refused`, `empty_rdf_snapshot_refused`)

### 2. SPARQL Smoke Queries

**Required Law:** `SEMANTIC_GRAPH_IS_QUERYABLE`

**Acceptance Criteria:**
- The following cold-path queries successfully execute against the semantic graph:
  - `list_documents`
  - `list_monikers`
  - `find_definitions_for_symbol`
  - `find_references_for_symbol`
  - `find_symbols_in_document`
  - `prove_named_graph_digest_binding`

**Rejection Criteria:**
- Smoke queries fail to execute or return empty results (`LSPMAX-SEMANTIC-GRAPH-SPARQL-SMOKE-FAILED` - Severity: `REFUSE`).

**Witness Tests Required:**
- `sparql_lists_documents`, `sparql_lists_monikers`, `sparql_finds_symbol_definitions`, `sparql_finds_symbol_references`, `sparql_proves_named_graph_digest_binding`
- Falsification test (`sparql_smoke_failure_refused`)

## Receipt Requirement
- `receipts/v26.6.28-lsif-oxigraph.receipt.json`

## Governing Equation
`R_B ⊢ A = μ(O*_B)`
