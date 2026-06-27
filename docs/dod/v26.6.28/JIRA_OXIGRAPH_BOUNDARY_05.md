# [JIRA-05] Step 5: Oxigraph Boundary

## Status
**ADMITTED**

## Definition of Done

This ticket governs the integration of Oxigraph into the `lsp-max` codebase. It explicitly forbids leaking Oxigraph internals into the general codebase and prohibits updating the semantic graph on the hot path.

### 1. Oxigraph Boundary

**Required Law:** `OXIGRAPH_BOUNDARY_HELD`

**Acceptance Criteria (TRUE):**
- All Oxigraph imports are strictly confined to `src/runtime/control_plane/semantic_graph/` and its submodules (`mod.rs`, `store.rs`, `named_graphs.rs`, `snapshot.rs`, `sparql.rs`, `lsif_import.rs`, `receipt.rs`).
- The public API exposes only typed domain objects.

**Rejection Criteria (FALSE):**
- `oxigraph::model::Term`, `oxigraph::store::Store`, or SPARQL internals appear outside `semantic_graph/`.

**Counterfactual Probe:**
- **Action:** Add an `oxigraph` import outside `semantic_graph/`.
- **Expected Result:** `LSPMAX-OXIGRAPH-BOUNDARY-BREACH` (Severity: `REFUSE`).

**Witnesses:**
- Import scan
- `cargo check`
- Boundary tests (`oxigraph_imports_confined_to_semantic_graph`, `oxigraph_boundary_breach_refused`)

**Repair Action:**
- Move Oxigraph usage behind `semantic_graph/` and expose a typed boundary object.

### 2. Hot-Path Refusal

**Required Law:** `OXIGRAPH_NOT_ON_HOT_PATH`

**Acceptance Criteria (TRUE):**
- `didOpen`, `didChange`, and `didSave` do not synchronously rebuild Oxigraph or the semantic graph.

**Rejection Criteria (FALSE):**
- `didChange` invokes `semantic_graph.refresh()`.
- `didChange` runs SPARQL smoke tests.
- `didChange` rebuilds RDF named graph.

**Counterfactual Probe:**
- **Action:** Force `semantic_graph.refresh()` inside `didChange`.
- **Expected Result:** `LSPMAX-OXIGRAPH-HOT-PATH-REFUSED` (Severity: `REFUSE`).

**Witnesses:**
- Call graph test
- Hot-path scan
- Runtime benchmark (`oxigraph_not_called_from_did_change`, `oxigraph_hot_path_counterfactual_refused`)

**Repair Action:**
- Move graph refresh to an explicit command, idle job, release audit, or cold-path task.

## Receipt Requirement
- `receipts/v26.6.28-oxigraph-boundary.receipt.json`

## Governing Equation
`R_B ⊢ A = μ(O*_B)`
