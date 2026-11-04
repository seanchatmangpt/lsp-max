# Lane LSPR — residual allowlist seeding (F16/W16 item 5)

Date: 2026-11-02. Repo: /Users/sac/lsp-max (canonical checkout, no worktrees).

## Seed consumption design

- Source of truth: cgap
  `docs/jira/v26.11.2/countermeasures/residuals.toml`
  (sha256 `cc62d41d9911e1f78be299327298322dcef62a0492aa7434e0fb42bf16d06409`).
  Copied byte-exact to `crates/branchless-lsp/residuals.toml` and embedded
  with `include_str!` in `attributes.rs`.
- Embedding justified over runtime path read: the seed must ship inside the
  LSP binary with zero runtime path lookup (an absent-file read would turn
  seeding into a silent skip); the byte-exact copy keeps the cgap provenance
  chain intact (sha256 verified post-copy).
- `ResidualGrid::from_str` parses via the `toml` crate (dep added to
  `Cargo.toml`); `ResidualGrid::canonical()` parses the embedded grid.
- `classify_seeded(symbol, target, attrs, branches_found)`: a grid-matched
  symbol is HINT/RESIDUAL-classified WITHOUT any attribute annotation
  (bare fn names match the crate-local path's last segment; target must
  match or be `any`); unknown symbols fall through to the attribute-only
  path, so unannotated branchy unknowns stay ErrorRefusedNoAttr.
- `ReceiptVector::fold_seeded(...)` folds per-function verdicts with the
  grid and carries each seeded residual's provenance
  (`kind={:?} branches=N provenance=<cgap receipt>:<line>`) into a
  side-channel `Vec<String>` (provenance is receipt metadata, not hashed
  into the BLAKE3 vector).

## Court grid (all ran this session, exit 0)

| Court | Mutation | Flip |
|---|---|---|
| `canonical_grid_parses_with_exact_row_count` | pinned 10 rows + branch_count cell | `branch_count = 9` tamper flips |
| `seeded_symbol_is_residual_without_annotation` | 7 (symbol,target) cells → HINT/RESIDUAL with zero attrs | removing grid rows flips |
| `unknown_symbol_still_refused_even_when_seeded_grid_present` | branchy unknown fn | classifies ErrorRefusedNoAttr |
| `skip_reference_rows_seed_with_negative_count` | asserts exactly 3 SKIP-REFERENCE rows | adding/removing a row flips |
| `tampered_branch_count_inflated_is_detected` | `1 → 9` inflation of select_slice/aarch64 | tampered grid != canonical; canonical cell pinned = 1 |
| `target_mismatch_does_not_seed` | x86_64 lookup on aarch64/rv32 grid | stays Unknown |
| `seeded_fold_counts_residual_and_carries_provenance` | select_slice(1)/cmov_u64(0)/unknown(2) | exact counts (1,1,1) + provenance cite present |
| `seeded_receipt_hash_flips_vs_unseeded_world` | same report ± grid | receipt vector flips (anti-vacuity) |

## Commands + exits (this session)

- `cargo test -p branchless-lsp` in `CARGO_TARGET_DIR=/tmp/lspr-target`:
  29 lib tests (21 existing + 8 new) + 0 + 2 + 0, 0 failures — exit 0.
- Seed copy sha256-verified byte-exact against the cgap source.

## Disclosed SLA unblock fixes (files outside my OWNERS)

1. `disasm_court.rs:217` `.into_iter()` on the capstone `extra_mode`
   slice — pre-existing compile break (E0277, `[T;1]` not an Iterator).
   Concurrently, another lane landed `use capstone::prelude::*` + the
   multi-section text fix; my into_iter composes with their fix. The
   multi-`.text.<fn>`-section judging fix in `judge()` is THEIRS, not
   mine (my larger patch was abandoned after their fix landed first).
2. `tests/riscv32imc_court.rs` probe: added `if x == 0 { return 0; }`
   guard to `rf_branchy` — the -O loop-only body produced no compressed
   branch mnemonic (`c.b*` absent), so the RVC court's compressed-branch
   premise never held. Verified empirically before patching: the guard
   yields `c.beqz`/`c.j` on riscv32imc (repro in /tmp/judge-repro).
   The branch-count courts' premise (branchy flags, free passes) still
   holds on the patched probe.

## Standing

ALIVE for the seed lane: every seeded cell, the tamper court, and the
unknown-refused path ran this session with exit 0 on the exact subject.
The pre-existing riscv integration court went from compile-abort to
2 passed, 0 failed. Perf numbers: none claimed.
