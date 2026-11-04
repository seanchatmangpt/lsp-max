# lsp-max: land or delete remote branch `claude/quirky-cerf-ghb67q`

- Standing: OPEN
- Created: 2026-09-19 (v26.9.19 gh survey wave)
- Source: remote branch `claude/quirky-cerf-ghb67q` — not merged into `master`, no open PR
- Evidence: `git branch -r --no-merged origin/master` lists it; absent from `gh pr list` heads

## Work to complete
- Decide: open a PR (`gh pr create -R seanchatmangpt/lsp-max --head claude/quirky-cerf-ghb67q`) or delete (`git push origin --delete claude/quirky-cerf-ghb67q`).
- If superseded, delete; otherwise land through review.

## Acceptance
- After `git fetch --prune`, `git branch -r --no-merged origin/master` no longer lists `claude/quirky-cerf-ghb67q`.

## History
- 2026-09-19 | OPEN | survey found PR-less unmerged branch | claude/quirky-cerf-ghb67q | decision pending
