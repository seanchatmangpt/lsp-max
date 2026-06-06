# Project: tower-lsp-max Oxigraph/SPARQL Control Plane Implementation

## Architecture
- Main workspace integrating Oxigraph 0.5.8 into `tower-lsp-max`.
- Ingestion boundary uses `RelationAdmitter` trait to transition graph snapshots through 7 states: `RAW`, `CANDIDATE`, `ADMITTED`, `REFUSED`, `QUARANTINED`, `SUPERSEDED`, `REPLAYED`.
- Evaluation of 5 core semantic invariants using SPARQL 1.1/1.2 ASK/SELECT queries on embedded `oxigraph::store::Store`.
- Materialized Views cache SPARQL projections asynchronously in concurrent in-memory `DashMap` structures to serve LSP queries in $O(1)$ time.
- Cryptographic Receipt Chain tracks transaction history by chaining BLAKE3 digests and signing with Ed25519.
- Deterministic Replay Engine verifies past transitions by mock-controlling clock/randomness.
- Typestate machine logic is enforced at compile time using generic `Machine<L, P, D>` container and affine ownership types.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1 | Milestone 1: Documentation Refinement & Finalization | Complete 8 PRD/ARD files, fix invariants/vocabulary, and pass syntax verification. | None | DONE |
| 2 | Milestone 2: Implement Admitted RDF Graph State & Oxigraph Integration (R1) | Implement `RelationAdmitter` trait, memory & RocksDB stores, and LSIF-to-RDF translator. | Milestone 1 | IN_PROGRESS |
| 3 | Milestone 3: Implement SPARQL Invariant Verification & Diagnostics (R2) | Implement 5 invariant SPARQL validation checks, diagnostics report, and quarantine handler. | Milestone 2 | PLANNED |
| 4 | Milestone 4: Implement Materialized Views & LSP Routing (R3) | Implement background materialized views (`DashMap`) and LSP hot-path query routing. | Milestone 3 | PLANNED |
| 5 | Milestone 5: Implement Cryptographic Receipt Chaining (R4) | Implement BLAKE3 hashing and Ed25519 signature receipt generation and verification. | Milestone 3 | PLANNED |
| 6 | Milestone 6: Implement Deterministic Replay Engine (R5) | Implement query consequence replay engine verifying receipts deterministically. | Milestone 5 | PLANNED |
| 7 | Milestone 7: Ostar Typestate Kernel Integration (R6) | Bind transitions to `Machine<L, P, D>` and `TypestateKernel` trait with affine ownership. | Milestone 2, 6 | PLANNED |
| 8 | Milestone 8: Verification Gate & Final Auditing | Verify clean clippy/tests, run E2E validation, and confirm Forensic Auditor clean status. | All Milestones | PLANNED |

## Interface Contracts
- `RelationAdmitter`: Ingests `Vec<Element>` and produces `Result<AdmittedRelationGraph, VerificationReport>`.
- `SparqlEvaluator`: Runs SPARQL queries on `oxigraph::store::Store`.
- `CryptographicReceipt`: Contains SHA256/BLAKE3 digests and Ed25519 signatures validating state transitions.

## Code Layout
- `crates/tower-lsp-max-base/src/abstractions.rs` -> Trait and type boundaries.
- `tower-lsp-max-runtime/src/control_plane/mod.rs` -> Main entry point for control plane.
- `tower-lsp-max-runtime/src/control_plane/admission.rs` -> Ingestion & admission typestate implementation.
- `crates/tower-lsp-max-lsif/src/` -> LSIF parser structures.
