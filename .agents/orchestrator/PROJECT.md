# Project: tower-lsp-max Workspace Integration

## Architecture
- Root workspace containing:
  - Existing crates: tower-lsp-max-macros, tower-lsp-max-protocol, tower-lsp-max-runtime, tower-lsp-max-agent
  - New crate: crates/tower-lsp-max-specgen (copied from ~/Downloads/tower-lsp-max-specgen)
- Outputs:
  - generated/lsp_minimal.rs (output from specification generator)
  - docs/adr/ADR-0001-tower-lsp-max-purpose.md
  - docs/law/law-state-protocol-frame.md
  - docs/reports/SPECGEN-001-bootstrap-report.md

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | R1: Copy and Organize Source Crate | Copy `tower-lsp-max-specgen` to `crates/tower-lsp-max-specgen` | None | DONE |
| 2 | R2: Workspace Cargo & Git Setup | Update Cargo.toml workspace members, check/configure gitignore | Milestone 1 | DONE |
| 3 | R3: Setup Documentation & Guidelines | Add ADR-0001, system framework guide, and start bootstrap report | Milestone 2 | DONE |
| 4 | R4: E2E Generation & Final Verification | Run cargo check/fmt/test, generate minimal LSP, finish bootstrap report | Milestone 3 | DONE |

## Interface Contracts
- None. This is a repository layout reorganization.

## Code Layout
- `crates/tower-lsp-max-specgen/` -> Specification generator crate source.
- `generated/` -> Output folder for generated Rust files.
- `docs/` -> ADRs, framework laws, reports.
