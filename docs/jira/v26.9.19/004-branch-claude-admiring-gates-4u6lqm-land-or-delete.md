# lsp-max: land or delete remote branch `claude/admiring-gates-4u6lqm`

- Standing: OPEN
- Created: 2026-09-19 (v26.9.19 gh survey wave)
- Source: remote branch `claude/admiring-gates-4u6lqm` — not merged into `master`, no open PR
- Evidence: `git branch -r --no-merged origin/master` lists it; absent from `gh pr list` heads

## Work to complete
- Decide: open a PR (`gh pr create -R seanchatmangpt/lsp-max --head claude/admiring-gates-4u6lqm`) or delete (`git push origin --delete claude/admiring-gates-4u6lqm`).
- If superseded, delete; otherwise land through review.

## Acceptance
- After `git fetch --prune`, `git branch -r --no-merged origin/master` no longer lists `claude/admiring-gates-4u6lqm`.

## History
- 2026-09-19 | OPEN | survey found PR-less unmerged branch | claude/admiring-gates-4u6lqm | decision pending
