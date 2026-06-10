# Quality & Gate Policies: lsp-max v26.6.5

This document details the multi-stage quality verification pipeline, risk management register, and binary gates for promoting release candidate packages to ALIVE status.

---

## 1. The 7-Stage Verification Ladder

To ensure robust functional behavior, high performance under load, and cryptographic reproducibility, the control plane is validated across a 7-stage verification ladder:

1. **Unit**: Validates physical parsing, IRI building, and namespace prefix mapping rules (e.g. `lsif:`, `prov:`, `max:`, `rcpt:`) to prevent namespace collisions.
2. **Integration**: Sets up an in-memory `oxigraph::store::Store` to verify triple ingestion streams and executes SPARQL invariant checks (such as checking for orphan LSIF relations or ontology laundering).
3. **E2E**: Spins up the full `lsp-max` LSP server process, indexes a sample workspace folder, and triggers JSON-RPC queries to verify that definition/reference lookup responses include correct cryptographic receipts.
4. **Chaos**: Feeds malformed, corrupt, or circular LSIF inputs into the ingestion boundary to confirm that the `RelationAdmitter` successfully rejects/quarantines invalid triples and outputs proper diagnostic refusal logs.
5. **Stress**: Evaluates system memory and CPU consumption while streaming large LSIF files (containing 1M+ triples). Verifies that the parser utilizes streaming iterators to prevent DOM heap exhaustion.
6. **Benchmark**: Measures query response times on the interactive data plane. The target latency for definition lookups from in-memory materialized views must stay below the `<5ms` threshold.
7. **Verifier Report**: Executes independent replay runs of SPARQL queries against identical snapshot states, validating that the output hashes match the values recorded in original cryptographic receipts.

---

## 2. Risk Register

The table below lists the six key architectural and security risks identified for the control plane and their corresponding mitigations:

| Risk ID | Title & Description | Severity | Mitigation Strategy |
| :--- | :--- | :--- | :--- |
| **RSK-001** | **SPARQL Performance Bottleneck**: Executing SPARQL queries during active LSP requests degrades definition/reference lookup latencies. | High | Hard architectural separation: Serving all interactive client requests from in-memory materialized views (`DashMap`), updating them asynchronously. |
| **RSK-002** | **Ontology Laundering**: Third-party plugins or custom rules masquerade their private metadata under the official `lsif:` prefix. | High | Ingestion-time prefix whitelisting inside `RelationAdmitter`, rejecting any unregistered properties inside public namespaces. |
| **RSK-003** | **LSIF-RDF Ontology Drift**: Structural changes to the physical LSIF schema mismatch the RDF predicate definitions. | High | Generating Rust structure models and RDF schema mappings from a single source definition using the `lsp-max-specgen` tool. |
| **RSK-004** | **Oxigraph Store Corruption**: Persistent RocksDB database locking conflicts or write failures corrupt graph snapshots. | Medium | Primary reliance on in-memory store mode for standard test suite execution; dedicated single-threaded writer thread pool with transaction boundaries. |
| **RSK-005** | **MCP/A2A Protocol Instability**: Future revisions to external model and agent interaction specifications break control plane compatibility. | Medium | Bounding projection views behind stable interface traits, insulating the internal RDF control plane from transport protocol changes. |
| **RSK-006** | **False ALIVE Status**: Promoting a release candidate without verifying cryptographic replay determinism. | Critical | Strict build pipeline gate requiring 100% matching verifier reports before a release can be certified as ALIVE. |

---

## 3. Release Gate Criteria

Promotion of a release candidate is evaluated against four distinct classification states:

- **ALIVE**:
  - *Criteria*: All 7 stages of the Verification Ladder pass, zero performance regressions are found (definition lookup latency `<5ms`), 100% of generated diagnostics/projections are bound to cryptographic receipts, and the verifier report confirms replay determinism.
- **PARTIAL_ALIVE**:
  - *Criteria*: The core Oxigraph store is fully functional and LSP queries pass, but cryptographic receipt verification is disabled, or independent verifier replays are missing.
- **BLOCKED**:
  - *Criteria*: Memory leaks or thread starvation detected during stress testing, or dependencies do not match licensing and security policies.
- **BUILD_BROKEN**:
  - *Criteria*: The Rust code fails to compile, or any of the existing LSIF/LSP baseline tests fail.
