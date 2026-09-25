# MAX-001 Initial Snapshot

## Current git HEAD
`479e7b564b328b047f0fa99234f2e0c65579a922`

## Dirty files before work
- `.agents/ORIGINAL_REQUEST.md`
- `.agents/sentinel/BRIEFING.md`
- `.agents/sentinel/handoff.md`
- `crates/tower-lsp-max-cli/src/nouns/agent.rs`
- `crates/tower-lsp-max-cli/src/nouns/diagnostics.rs`
- `crates/tower-lsp-max-cli/src/nouns/plugin.rs`
- `crates/tower-lsp-max-cli/src/nouns/server.rs`
- `crates/tower-lsp-max-cli/src/nouns/state.rs`
- `crates/tower-lsp-max-cli/src/nouns/telemetry.rs`

## Existing crates
- `tower-lsp-max-macros`
- `tower-lsp-max-protocol`
- `tower-lsp-max-runtime`
- `tower-lsp-max-agent`
- `crates/tower-lsp-max-specgen`
- `crates/tower-lsp-max-cli`

## Existing generated files
- `generated/lsp_3_18.rs`
- `generated/lsp_minimal.rs`
- `tower-lsp-max-protocol/src/generated_3_18.rs`
- `tower-lsp-max-protocol/src/lsp_3_18.rs`

## Existing docs
- `docs/adr/ADR-0001-tower-lsp-max-purpose.md`
- `docs/law/law-state-protocol-frame.md`
- `docs/reports/LSP-3.18-RESEARCH.md`
- `docs/reports/SPECGEN-001-bootstrap-report.md`

## Agent assignments
1. `max_coordinator`: Sequencer, conflict control, final report.
2. `specgen_metamodel_agent`: Ingest meta-model fixture, generator correctness.
3. `generated_protocol_agent`: Generated Rust protocol vocabulary hygiene.
4. `lsp_surface_comparator_agent`: Compare protocol coverage against LSP 3.18 surface.
5. `max_protocol_agent`: Custom max/* protocol surface.
6. `law_state_runtime_agent`: Law-state runtime model.
7. `transaction_repair_agent`: Transactional code actions and repair plans.
8. `cli_surface_agent`: CLI command surface.
9. `docs_law_agent`: Docs-as-release-law.
10. `verifier_agent`: Machine verification, global cargo lock gate runner.

## Cargo command lock policy
- Absolute Concurrency Law: Only `verifier_agent` runs global cargo commands (`cargo fmt`, `cargo check`, `cargo test`, `cargo clippy`). No other agent may run those commands.
- Under NO circumstances should any other agent run global cargo commands.

## Final status
MAX_IMPLEMENTATION_PARTIAL
