# lsp-max: land or delete remote branch `agent/protocol-court-checkpoints`

- Standing: OPEN
- Created: 2026-09-19 (v26.9.19 gh survey wave)
- Source: remote branch `agent/protocol-court-checkpoints` — not merged into `master`, no open PR
- Evidence: `git branch -r --no-merged origin/master` lists it; absent from `gh pr list` heads

## Work to complete
- Decide: open a PR (`gh pr create -R seanchatmangpt/lsp-max --head agent/protocol-court-checkpoints`) or delete (`git push origin --delete agent/protocol-court-checkpoints`).
- If superseded, delete; otherwise land through review.

## Acceptance
- After `git fetch --prune`, `git branch -r --no-merged origin/master` no longer lists `agent/protocol-court-checkpoints`.

## History
- 2026-09-19 | OPEN | survey found PR-less unmerged branch | agent/protocol-court-checkpoints | decision pending
