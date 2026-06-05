## 2026-06-04T17:14:31-07:00
Your working directory is `/Users/sac/tower-lsp-max/.agents/teamwork_preview_worker_m3`.
Your identity is teamwork_preview_worker.
Your task is to:
1. Create the following directories:
   - `/Users/sac/tower-lsp-max/docs/adr`
   - `/Users/sac/tower-lsp-max/docs/law`
   - `/Users/sac/tower-lsp-max/docs/reports`
   - `/Users/sac/tower-lsp-max/generated`
2. Write the ADR document `docs/adr/ADR-0001-tower-lsp-max-purpose.md` explaining the decision to bootstrap the specification generator first rather than writing types by hand. It should follow standard ADR format (Title, Status, Context, Decision, Consequences).
3. Write the system framework guide `docs/law/law-state-protocol-frame.md` explaining the planned design space, specifically defining the roles of the protocol (raw generated types), server (handler traits), runtime (asynchronous I/O execution), and law plugins (runtime validation/policy checkers).
4. Run the generator to produce `generated/lsp_minimal.rs` from `crates/tower-lsp-max-specgen/fixtures/minimal-metaModel.json` using the command:
   `cargo run -p tower-lsp-max-specgen -- --input crates/tower-lsp-max-specgen/fixtures/minimal-metaModel.json --output generated/lsp_minimal.rs`
   Verify that this command exits successfully (code 0) and creates a valid Rust file at `generated/lsp_minimal.rs`.
5. Write `docs/reports/SPECGEN-001-bootstrap-report.md` capturing the environment details, file list of integrated files, verification command output, and next steps.
6. Verify that `cargo fmt --check`, `cargo check --workspace`, and `cargo test --workspace` all compile/pass successfully.
7. Write your handoff report in `/Users/sac/tower-lsp-max/.agents/teamwork_preview_worker_m3/handoff.md` detailing the actions taken and verification outcomes.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
