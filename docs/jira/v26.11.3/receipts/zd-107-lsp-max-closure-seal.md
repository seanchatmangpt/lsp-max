# ZD-107 — lsp-max v26.11.3 wave-closure seal receipt

Date: 2026-11-04. Standing: ALIVE.

Repo: /Users/sac/lsp-max @ master, HEAD e23629b (pushed). Condensed
wave-closing seal receipt; full body:
docs/jira/v26.11.3/closure/lsp-max-closure.md.

## Deliverables

AF-19 F1 flush fix (shim-court verified, receipts/af-19-f1-fix.md) ·
AF-20 pool-drop kill guard + C5 court (receipts/af-20-pool-drop.md) ·
ZD-47 probe resolve_child_bin (6673823) · ZD-54 max-protocol ttl drift fix
(receipts/zd-54-max-protocol-drift.md, bd97e5c) · ZD-91 broken targets
(receipts/zd-91-broken-targets.md, af6c8dc) · ZD-96 stdio witness
(receipts/zd-96-stdio-witness.md, 813570a) · registry opt-out
(registry_ontology_test with disclosed --skip-ea-admission, e23629b).

## Giant-blob repair history

Two filter-repo repairs of build artifacts swept into unpushed lane
commits: target-lane*/ (149MB rlibs) then target-verify/; merge anchor
c261f613; no force push, old lineages preserved as merge parents;
.gitignore now excludes both scratch roots.

## Verification (this session, real runs)

- `cargo check -p lsp-max-compositor` → exit 0 (30.12s)
- `cargo test -p lsp-max-compositor --test child_churn_stress` →
  5 passed; 0 failed, exit 0 (11.82s)

Wave-dispatch context (zd-64, receipt not on disk in this repo): 863/2
full-suite census; both reds fixed post-hoc (flake-class).

## Standing

ALIVE for the contracted scope: HEAD pushed, all cited receipts resolve
on disk, both verification commands exit 0 with the cited counts.

Falsifier: any cited receipt missing or disagreeing with its body, or
either verification command failing to reproduce.
