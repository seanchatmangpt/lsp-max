# Original User Request

## Initial Request — 2026-06-04T17:10:49-07:00

Convert the downloaded `tower-lsp-max-specgen` scaffold into a Rust workspace layout at `~/tower-lsp-max` while preserving existing workspace crates (`tower-lsp-max-macros`, `tower-lsp-max-protocol`, `tower-lsp-max-runtime`, `tower-lsp-max-agent`).

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Copy and Organize Source Crate
- Copy the `tower-lsp-max-specgen` source crate from `~/Downloads/tower-lsp-max-specgen` to `/Users/sac/tower-lsp-max/crates/tower-lsp-max-specgen`.
- Rationale: Structure the workspace cleanly without affecting the existing root level crates.

### R2. Workspace Initialization and Cargo/Git Setup
- Update `/Users/sac/tower-lsp-max/Cargo.toml` to include `"crates/tower-lsp-max-specgen"` in the workspace members.
- The workspace members list must look like:
  ```toml
  [workspace]
  members = [
      ".",
      "./tower-lsp-max-macros",
      "./tower-lsp-max-protocol",
      "./tower-lsp-max-runtime",
      "./tower-lsp-max-agent",
      "crates/tower-lsp-max-specgen",
  ]
  ```
- Ensure workspace lint rules, package metadata, and edition are correctly set.
- Ensure Git is initialized and `.gitignore` matches required files (e.g., target, generated/ files, and logs).

### R3. Setup Documentation and Architecture Guidelines
- Create ADR document `docs/adr/ADR-0001-tower-lsp-max-purpose.md` explaining decision to bootstrap generator first.
- Create system framework guide `docs/law/law-state-protocol-frame.md` explaining planned design space (protocol, server, runtime, law plugins).
- Write `docs/reports/SPECGEN-001-bootstrap-report.md` capturing environment, file list, verification command output, and next steps.

### R4. Verification and Sample Generation
- Ensure workspace formatting and type correctness (`cargo fmt --check`, `cargo check --workspace`, `cargo test --workspace`).
- Run the generator to produce `generated/lsp_minimal.rs` from `crates/tower-lsp-max-specgen/fixtures/minimal-metaModel.json`.

## Acceptance Criteria

### Project Verification and Compilation
- [ ] `cargo check --workspace` compiles successfully with no workspace check errors.
- [ ] `cargo fmt --check` passes successfully.
- [ ] `cargo test --workspace` executes successfully.
- [ ] Generator command `cargo run -p tower-lsp-max-specgen -- --input crates/tower-lsp-max-specgen/fixtures/minimal-metaModel.json --output generated/lsp_minimal.rs` exits successfully.
- [ ] File `generated/lsp_minimal.rs` exists, and its top lines show valid Rust code (inspected via standard CLI or reading).
- [ ] Artifact `docs/reports/SPECGEN-001-bootstrap-report.md` exists and contains the requested status and commands table.
