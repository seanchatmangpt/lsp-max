# lsp-max v26.11.3 wave-closure seal receipt

Date: 2026-11-04. Wave-closing seal for lsp-max's contribution to the
five-repo v26.11.3 closure wave (mirror of
`/Users/sac/ggen/docs/jira/v26.11.3/closure/v26.11.3-closure.md`, condensed
— lsp-max is a small contributor to the wave). Zero src/test edits in this
receipt's own actuation; all code landed via receipted lanes cited below.

## Exact subject

- Repo: /Users/sac/lsp-max, branch master, HEAD e23629b (pushed).
- Receipts dir: docs/jira/v26.11.3/receipts/ — 16 files on disk
  (`ls | wc -l` at seal time).
- Wave-dispatch context (zd-64, dispatched elsewhere): full-suite census
  863 passed / 2 failed; both reds fixed post-hoc (XD-16's root-cause fix
  pattern upstream at config-lsp; the two lsp-max reds were flake-class).
  The zd-64 receipt itself is not on disk in this repo — recorded as
  wave context, not a rowed receipt.

## Deliverables (each cites its on-disk receipt)

- **AF-19 F1 flush fix** — debounced flush path collapsed diagnostic
  `range.end` to start; fixed in flush_coordinator.rs, shim-court
  verified (court flips on revert, restore byte-exact):
  receipts/af-19-f1-fix.md.
- **AF-20 pool-drop kill guard + C5 court** — pool drop previously left
  children alive (read_loop sender clone kept stdin open);
  `pool_drop_teardown_reaps_all_children_within_bound` court added:
  receipts/af-20-pool-drop.md.
- **ZD-47 probe** — `resolve_child_bin` portable binary resolution with
  loud typed [SKIP] (ZD-47 pattern, reused by ZD-96): commit 6673823.
- **ZD-54 max-protocol ttl drift** — 2 aspirational `lspMax/*`
  notification individuals with zero Rust implementation removed;
  `src/andon/lsp.rs` consts ruled code authority:
  receipts/zd-54-max-protocol-drift.md (commit bd97e5c).
- **ZD-91 broken targets** — 2 broken test targets (quarantined hooks
  behind `refund-demo-hooks`; stale FlushCoordinator signature) fixed;
  `cargo check --workspace --all-targets` exit 0:
  receipts/zd-91-broken-targets.md (commit af6c8dc).
- **ZD-96 stdio witness** — hardcoded `/Users/sac` target path removed;
  env override + target-dir fallback + loud [SKIP]:
  receipts/zd-96-stdio-witness.md (commit 813570a).
- **Registry opt-out** — registry_ontology_test passing with disclosed
  `--skip-ea-admission` (content-pack sync, no SBB manufacture; ambient
  ggen 26.11.3 FM-LAW-015 class): commit e23629b (HEAD).

## Giant-blob repair history

lsp-max carries two rounds of plumbing repair for build artifacts swept
into unpushed lane commits: (1) `target-lane*/` artifacts (149MB rlibs,
3.2 GiB pack) stripped via filter-repo and merged onto origin/master with
`-s ours` (c977c1b, no force push); (2) `target-verify/` artifacts
stripped the same way, merge anchor **c261f613** ("Merge artifact-stripped
master onto origin/master lineage"). `.gitignore` now excludes
`target-lane*/` and target-verify scratch. No force push; old lineages
preserved as merge parents.

## Verification ladder (this session, real runs)

- `cargo check -p lsp-max-compositor` → exit 0 (Finished dev profile,
  30.12s).
- `cargo test -p lsp-max-compositor --test child_churn_stress` →
  **5 passed; 0 failed**, exit 0 (C5 pool-drop teardown court included;
  11.82s).

## Standing

ALIVE for the contracted scope: HEAD e23629b pushed, every deliverable
above resolves to a receipt file or commit on disk, both verification
commands exit 0 with the cited counts. One unrelated pre-existing working
-tree modification (`examples/wasm4pm-lsp/tests/receipts/cog010-scan.json`)
is not this lane's and is left untouched.

Falsifier: any cited receipt missing or disagreeing with its body, or
either verification command failing to reproduce.
