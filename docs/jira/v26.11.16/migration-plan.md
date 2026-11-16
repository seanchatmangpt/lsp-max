# lsp-max migration plan v26.11.16

Standing: UNVERIFIED

## Identity
- Path: /Users/sac/lsp-max
- Remote: https://github.com/seanchatmangpt/lsp-max (upstream origin/master)
- Branch: master@86ad66d, last commit 2026-11-04, 2509 commits, 12143 MB (.git = 12 GB)
- Languages/build: Rust workspace (Cargo.toml, crates/, Justfile), python/, ontology/, Docker
- Nested repos: none

## Observed state
- Dirty 1: lsp-max.toml; untracked 0
- Ahead 0 / behind 0; stash count 998; 44 unpushed local branches
- Build dirs: none (no target/); working tree small — artifacts 48K, generated 28K,
  playground 212K; the 12 GB is entirely .git (stashes/branches/history, lsif_dump.json?)
- Submodules: none. docs/jira has v26.11.3, v26.11.4 newer than census-listed v26.9.19.

## What it is
Fork of tower-lsp upgraded into lsp-max, a "law-state runtime projected through LSP" with
maximal LSP 3.18 coverage and a custom max/* protocol (receipts, gates, conformance vectors).
AGENTS.md is the enforced constitution.

## Classification
ACTIVE (census agrees; commits two weeks old, CI green-path work).

## Migration risk
HIGH — 998 stashes and 44 unpushed branches exist only in the 12 GB local .git; master itself
is pushed. lsp-max.toml edit is minor.

## Recommended action
COMMIT-PUSH
1. `cd /Users/sac/lsp-max && git diff lsp-max.toml`; DECISION keep → `git commit -am "chore:
   lsp-max.toml"` and `git push`.
2. `git branch -vv | grep -v '\[origin'` → DECISION push the 44 branches or bundle.
3. Stashes cannot push: `git bundle create /Volumes/ARCHIVE/lsp-max-full.bundle --all` and
   `git stash list > /Volumes/ARCHIVE/lsp-max-stashes.txt`. DECISION: most of 998 stashes
   are likely lane scratch; consider dropping after bundling.
4. `du -sh .git/objects` and `git count-objects -vH` to confirm what the 12 GB is before any
   `git gc` (gc is state-changing: defer to after migration).
5. Do NOT rsync 12 GB; re-clone on the new machine (shallow if needed).

## First commands on the new machine
`git clone https://github.com/seanchatmangpt/lsp-max && cd lsp-max && cargo check --workspace`
(then `just check` if defined)

## See Also
- ../v26.11.4/ (newest), ../v26.9.19/
- /Users/sac/docs/jira/v26.11.16/README.md
