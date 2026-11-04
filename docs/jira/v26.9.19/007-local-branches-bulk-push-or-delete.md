# lsp-max: triage 36 local-only branches

- Standing: OPEN
- Created: 2026-09-19 (v26.9.19 gh survey wave)
- Source: local branches with commits not on `origin/master` and no upstream
- Evidence: `git rev-list --count origin/master..<branch>` > 0 for each: feat/wire-fitness-into-andon-gate:1 fix/test-analyzer-workspace-membership:1 worktree-wf_4bccc165-37d-5:1 worktree-wf_76264338-36b-8:1 worktree-wf_935f81b8-ce6-1:1 worktree-wf_935f81b8-ce6-10:1 worktree-wf_935f81b8-ce6-11:1 worktree-wf_935f81b8-ce6-12:1 worktree-wf_935f81b8-ce6-13:1 worktree-wf_935f81b8-ce6-14:1 worktree-wf_935f81b8-ce6-17:1 worktree-wf_935f81b8-ce6-2:1 worktree-wf_935f81b8-ce6-23:2 worktree-wf_935f81b8-ce6-3:1 worktree-wf_935f81b8-ce6-4:1 worktree-wf_935f81b8-ce6-5:2 worktree-wf_935f81b8-ce6-6:1 worktree-wf_935f81b8-ce6-7:1 worktree-wf_935f81b8-ce6-8:1 worktree-wf_935f81b8-ce6-9:1 worktree-wf_9429e663-5e6-6:1 worktree-wf_9d422de2-c1e-1:1 worktree-wf_9d422de2-c1e-2:2 worktree-wf_9d422de2-c1e-3:1 worktree-wf_9d422de2-c1e-4:1 worktree-wf_9d422de2-c1e-5:1 worktree-wf_9d422de2-c1e-6:1 worktree-wf_a1289c9d-33f-2:1 worktree-wf_a1289c9d-33f-3:1 worktree-wf_a1289c9d-33f-4:1 worktree-wf_a6490eec-0c6-2:1 worktree-wf_bc18add5-00e-4:1 worktree-wf_df4c847c-936-5:1 worktree-wf_df4c847c-936-7:1 worktree-wf_e3ab33cb-bbf-7:1 worktree-wf_e3ab33cb-bbf-8:1

## Work to complete
- Triage each branch: push (`git push -u origin <branch>`) or delete after confirming the commits are obsolete/recoverable. Batch the work; record decisions in History.

## Acceptance
- Every listed branch is pushed or deleted; no local-only branch with unique commits remains.

## History
- 2026-09-19 | OPEN | survey found 36 local-only branches | full list above | triage pending
