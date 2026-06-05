# Project: tower-lsp-max Refactoring

## Architecture
- tower-lsp-max: Main language server library. Exposes JSON-RPC layer, state machine, and registry.
- tower-lsp-max-runtime: Core typestate machine, phases, and transition logic.
- tower-lsp-max-protocol: Declares custom methods (`max/*`), capability vectors, receipts, and analysis bundle structures.
- tower-lsp-max-cli: Actuation grammar (noun/verb CLI surface using `clap-noun-verb`).
- E2E / Integration Tests: Located in `tests/` and unit tests inline.

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| 1 | Core Test Helpers & Quick-wins | Extract common duplex-stream test helpers and refactor `tests/test_max_rpc_zero_coverage.rs`. | None | DONE |
| 2 | Playground AST Handlers | Refactor completions and diagnostics handlers into modules under 500 LOC. | M1 | PLANNED |
| 3 | Protocol Core Library | Refactor `tower-lsp-max-protocol/src/lib.rs` into modular sub-files. | M1 | PLANNED |
| 4 | Runtime Library | Extract inline test modules and modularize `tower-lsp-max-runtime/src/lib.rs`. | M1 | PLANNED |
| 5 | tower-lsp-max Core Library | Refactor `src/lib.rs`, `src/service.rs`, and `src/service/client.rs` to keep them <= 500 LOC. | M3, M4 | PLANNED |
| 6 | Integration Tests | Split integration tests in `tests/` into smaller files. | M1 | PLANNED |
| 7 | Code Generator & Generated Code | Refactor generator renderer and split generated schemas to be <= 500 LOC. | M1, M5 | PLANNED |

## Code Layout
- Keep files <= 500 LOC.
- Organize submodules in subdirectories matching the module name (e.g. `src/service/client/` or similar).
