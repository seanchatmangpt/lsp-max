## 2026-06-05T00:12:42Z
Your working directory is `/Users/sac/tower-lsp-max/.agents/teamwork_preview_worker_m1_m2`.
Your identity is teamwork_preview_worker.
Your task is to:
1. Copy the `tower-lsp-max-specgen` crate source from `/Users/sac/Downloads/tower-lsp-max-specgen` to `/Users/sac/tower-lsp-max/crates/tower-lsp-max-specgen`. Keep all the subfolders/files intact (e.g. src/main.rs, src/metamodel.rs, src/render.rs, fixtures/minimal-metaModel.json, README.md, .gitignore, Cargo.toml).
2. Update `/Users/sac/tower-lsp-max/Cargo.toml` so that the members array under [workspace] looks exactly like:
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
3. Update `/Users/sac/tower-lsp-max/.gitignore` to make sure it includes targets, generated files, and logs. It should look like:
```gitignore
**/target
**/*.rs.bk
Cargo.lock
generated/
*.log
```
4. Run `cargo check -p tower-lsp-max-specgen` and `cargo test -p tower-lsp-max-specgen` to verify that the newly added crate builds and runs cleanly.
5. Provide a summary of the files copied, configuration changes made, and cargo command execution results.
6. Write a handoff report in `/Users/sac/tower-lsp-max/.agents/teamwork_preview_worker_m1_m2/handoff.md`.

MANDATORY INTEGRITY WARNING:
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A Forensic Auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.
