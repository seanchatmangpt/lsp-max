# Project: tower-lsp-max AMI Mesh Implementation

## Architecture
- tower-lsp-max: Main language server library. Exposes JSON-RPC layer, state machine, and registry.
- tower-lsp-max-runtime: Core typestate machine, phases, and transition logic. We will implement the formal LSP model and hooks here.
- tower-lsp-max-protocol: Declares custom methods (`max/*`), capability vectors, receipts, and analysis bundle structures.
- tower-lsp-max-cli: Actuation grammar (noun/verb CLI surface using `clap-noun-verb`).
- E2E / Integration Tests: Located in `tests/test_autonomic_mesh.rs` demonstrating the multi-instance mesh.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Formal Representation & Hooks | Design and implement LSP_i state model, H_i Hook trait, and registry in `tower-lsp-max-runtime` and `tower-lsp-max-protocol`. | None | PLANNED |
| 2 | 5-Layer Autonomic Architecture | Implement Grammar (CLI), Local Surface (JSON-RPC), Law-State Runtime, Hook Layer, and Autonomic Mesh controller. | M1 | PLANNED |
| 3 | Customer Service Case & E2E Tests | Implement proof case workflow and mesh integration tests in `tests/test_autonomic_mesh.rs`. | M2 | PLANNED |
| 4 | Workspace Verification & Audit | Workspace verification (cargo check, test, clippy, fmt) and Forensic Audit. | M3 | PLANNED |

## Interface Contracts
### Hook H_i ↔ LSP State Machine
- Event types: state transitions, diagnostics, repair plans, and receipts.
- Trigger: when registry/state transitions or emits diagnostics/receipts, look up and execute registered Hooks.
- Mesh communication: Hook execution in one LSP instance can trigger RPC calls or state changes in another LSP instance.

