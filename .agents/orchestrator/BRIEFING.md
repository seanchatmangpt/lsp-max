# BRIEFING — 2026-06-04T17:11:01-07:00

## Mission
Convert the tower-lsp-max-specgen scaffold into a Rust workspace layout, initialize, document, generate minimal LSP, and verify all requirements.

## 🔒 My Identity
- Archetype: teamwork_preview_orchestrator
- Roles: orchestrator, user_liaison, human_reporter, successor
- Working directory: /Users/sac/tower-lsp-max/.agents/orchestrator
- Original parent: main agent
- Original parent conversation ID: 77bb3455-05a2-4729-a732-ec31ca1017dd

## 🔒 My Workflow
- **Pattern**: Project
- **Scope document**: /Users/sac/tower-lsp-max/.agents/orchestrator/PROJECT.md
1. **Decompose**: Decompose the workspace migration, cargo setup, documentation, and verification into milestones.
2. **Dispatch & Execute** (pick ONE):
   - **Direct (iteration loop)**: Iterate using Explorer -> Worker -> Reviewer -> Challenger -> Auditor cycles.
   - **Delegate (sub-orchestrator)**: Spawn a sub-orchestrator for E2E tests, and milestones.
3. **On failure** (in this order):
   - Retry: nudge stuck agent or re-send task
   - Replace: spawn fresh agent with partial progress
   - Skip: proceed without (only if non-critical)
   - Redistribute: split stuck agent's remaining work
   - Redesign: re-partition decomposition
   - Escalate: report to parent (sub-orchestrators only, last resort)
4. **Succession**: Self-succeed at spawn count >= 16.
- **Work items**:
  1. Initialize project structure and PROJECT.md [done]
  2. Implement R1 (Copy Source Crate) [done]
  3. Implement R2 (Workspace/Cargo/Git Setup) [done]
  4. Implement R3 (Documentation & ADRs) & Fix Deserialization [in-progress]
  5. Implement R4 (Verification & Sample Generation) [pending]
- **Current phase**: 3
- **Current focus**: Implement R3 (Documentation & ADRs) & Fix Deserialization

## 🔒 Key Constraints
- Preserve existing workspace crates (tower-lsp-max-macros, tower-lsp-max-protocol, tower-lsp-max-runtime, tower-lsp-max-agent).
- Copy tower-lsp-max-specgen to crates/tower-lsp-max-specgen.
- Update Cargo.toml members specifically to standard layout.
- Generate generated/lsp_minimal.rs using correct generator command.
- Generate bootstrap report docs/reports/SPECGEN-001-bootstrap-report.md.
- Ensure all cargo check/fmt/test run cleanly.
- Never write code/run commands directly.
- The auditor must verify the implementation.

## Current Parent
- Conversation ID: 77bb3455-05a2-4729-a732-ec31ca1017dd
- Updated: not yet

## Key Decisions Made
- Use Project Orchestrator pattern.
- Milestone 1: Initialize layout, setup project files.
- Milestone 2: Setup tests & E2E framework.
- Milestone 3: Implement copying and layout updates (R1, R2).
- Milestone 4: Add documentation & law framework files (R3).
- Milestone 5: Verify & generate minimal LSP (R4).

## Team Roster
| Agent | Type | Work Item | Status | Conv ID |
|-------|------|-----------|--------|---------|
| Explorer 1 | teamwork_preview_explorer | Explore ~/Downloads/tower-lsp-max-specgen | completed | 4acca3d3-bc50-4d11-b760-8cb7d472ea16 |
| Explorer 2 | teamwork_preview_explorer | Explore ~/Downloads/tower-lsp-max-specgen | completed | 954ffd1d-1810-4fa1-930c-7263617938f7 |
| Explorer 3 | teamwork_preview_explorer | Explore ~/Downloads/tower-lsp-max-specgen | redundant | 9925cb11-7fd2-40ae-a532-ae1b006ac710 |
| Worker 1 | teamwork_preview_worker | Copy specgen and update Cargo/Git config | completed | a21a5f4c-2070-41c4-8349-736da70cd352 |
| Worker 2 | teamwork_preview_worker | Write docs, ADRs, run generator, verify workspace | completed | e4daec15-fb50-4036-aa78-ad05a3e60e8d |
| Auditor 1 | teamwork_preview_auditor | Run forensic audit and checks | completed | 0cfe91a6-4d65-49fa-baa6-ee9b75ac0e64 |

## Succession Status
- Succession required: no
- Spawn count: 6 / 16
- Pending subagents: none
- Predecessor: none
- Successor: not yet spawned

## Active Timers
- Heartbeat cron: killed
- Safety timer: killed
- On succession: kill all timers before spawning successor
- On context truncation: run `manage_task(Action="list")` — re-create if missing

## Artifact Index
- /Users/sac/tower-lsp-max/.agents/orchestrator/PROJECT.md — Project scope and milestone tracker
- /Users/sac/tower-lsp-max/.agents/orchestrator/progress.md — Progress log and liveness heartbeat
- /Users/sac/tower-lsp-max/.agents/orchestrator/BRIEFING.md — Persistent memory state
