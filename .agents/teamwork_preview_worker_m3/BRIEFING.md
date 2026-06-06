# BRIEFING — 2026-06-05T15:15:00-07:00

## Mission
Implement Milestone 3 (Materialized Views & LSP Routing) and Milestone 5 (Deterministic Replay Engine & End-to-End Verification) in the tower-lsp-max workspace.

## 🔒 My Identity
- Archetype: teamwork_preview_worker
- Roles: implementer, qa, specialist
- Working directory: /Users/sac/tower-lsp-max/.agents/teamwork_preview_worker_m3
- Original parent: 92da60fd-87f6-4193-8bdc-9b96270be182
- Milestone: Milestone 3 & 5 Implementation

## 🔒 Key Constraints
- CODE_ONLY network mode: no external HTTP/curl/wget.
- No dummy/facade implementations, no hardcoded results. All code must maintain real state.
- Write only to our own agents/teamwork_preview_worker_m3 folder (except when explicitly instructed to write to specified workspace locations).
- We are explicitly instructed to write to paths in the workspace: docs/adr, docs/law, docs/reports, and generated.

## Current Parent
- Conversation ID: 0b3b9120-c03c-443a-bbf5-38bdf2129619
- Updated: 2026-06-05T15:15:00-07:00

## Task Summary
- **What to build**:
  - `tower-lsp-max-runtime/src/control_plane/views.rs` with `MaterializedViewStore`, `update_views` querying SPARQL, and O(1) lookups.
  - `tower-lsp-max-runtime/src/control_plane/replay.rs` with `ReplayVerifier` and `verify_replay`.
  - Expose in `tower-lsp-max-runtime/src/control_plane/mod.rs` and `src/lib.rs`.
  - Wire materialized views to LSP lookup handlers or add integration tests.
  - Verify `scratch/verify_prd_ard.py` runs successfully.
  - Ensure cargo check/test/clippy pass 100%.
- **Success criteria**:
  - Code compiles, tests pass, scratch/verify_prd_ard.py runs 100% successfully.
- **Interface contracts**: `/Users/sac/tower-lsp-max/PROJECT.md`
- **Code layout**: standard rust crates layout

## Change Tracker
- **Files modified**: None yet
- **Build status**: [TBD]
- **Pending issues**: None yet

## Quality Status
- **Build/test result**: [TBD]
- **Lint status**: [TBD]
- **Tests added/modified**: None yet

## Loaded Skills
- None loaded.

## Key Decisions Made
- [TBD]

## Artifact Index
- /Users/sac/tower-lsp-max/.agents/teamwork_preview_worker_m3/ORIGINAL_REQUEST.md — Original task prompt
