# Lane WG2 — Tier 2 did_save wiring (F6's named blocker)

Status: ALIVE
Date: 2026-11-02
Repo: /Users/sac/lsp-max (branchless-lsp crate)

## Diff (owned files only)

### crates/branchless-lsp/src/rule_pack.rs (NEW)

- `tier2_pack()`: populates the rule pack set (was EMPTY, lib.rs:57 —
  F6's blocker) with one rule `ANDON-REFUSED-001` (severity error). The
  rule's regex surface is pinned to the never-matching `\b\B` — the
  authoritative evaluation is the Tier-2 court in did_save, never the
  Tier-1 regex scan (an empty pattern floods one diagnostic per
  character; witnessed 493 diagnostics before the pin).
- `run_tier2(source, target)`: compile_debuginfo (E9) -> judge_classified
  -> per-fn attrs (fn_attr_flags) -> ResidualGrid::classify_seeded ->
  ReceiptVector. Verdict law: branchy fn without residual allowance
  (attribute or seed grid) -> refusal; branch-free or residual -> clean.
- `sanitize_for_compile`: blanks `#[allow_isel_residual(...)]` /
  `#[branchless_invariant]` spans with spaces (byte-count-preserving —
  DWARF line/col stay exact) before rustc: rustc refuses unknown
  attributes (E0438-class, witnessed compile abort), and these attrs are
  ours, not rustc's.
- Mach-O symbol normalization: symbols arrive `_name`; bare-name
  (underscore-stripped) is used for the attr window and seed-grid
  lookup (underscore broke both lookups — witnessed flip).
- `verdict_diagnostics`: one ERROR `ANDON-REFUSED-001` diagnostic per
  classified branch site at the DWARF-mapped span (1-based DWARF ->
  0-based LSP), fallback span when no row; carries fn + classification
  in `data`.
- 10 tests: attr window, same-line multi-attr extraction, pack shape,
  branchy-refused-at-mapped-line, clean-admitted-zero-diags,
  residual-clean, seeded-select_slice-residual (seed grid consumed from
  LSPR), refusal-flips-off-with-allowance (anti-vacuity: the same
  branchy body + attr flips refusal -> residual; a plain `if` probe was
  correctly admitted — csel lowers branchless — so the flip court uses a
  while body that emits a real conditional), sanitize byte-exactness x2.

### crates/branchless-lsp/src/lib.rs

- packs: `ValidatedRulePackSet::new(&[tier2_pack()])` (was `&[]`).
- New Tier2State (Mutex) holding the last court ReceiptVector + uri.
- did_save wired: content (params.text, index fallback) ->
  run_tier2(host target) -> publishDiagnostics -> index conformance ->
  live vector. No Process.sleep anywhere (compile is synchronous; the
  client consumes the publish notification).
- max/conformanceVector overridden to serve from the live vector:
  refused=[ANDON_REFUSED] + score 0.0 when n_refused>0; admitted +
  score 100.0 when clean/residual; strict_mode true.
- 63 lib tests pass (was 53); full crate: 63+5+6+3 = 77 tests, exit 0.

### python/lsp_client/branchless_client.py (F6-owned — DISCLOSED UNBLOCK)

Minimal unblock to another lane's in-flight file under the freeze law,
disclosed here for BOTH lanes:
1. conformance_vector(): params `None` (null) instead of `{}` — the
   server's Option<SnapshotId> rejects a map (-32602 invalid type: map,
   expected a string; witnessed).
2. diagnostics_for_files(): drain the did_open publish before did_save —
   the Tier-1 advisory scan publishes immediately; the court verdict is
   the publish AFTER did_save. Without the drain the client returned the
   empty advisory publish and n_refused read 0 on branchy input.

## Witness commands (this session)

- CARGO_TARGET_DIR=/tmp/wg2-target cargo test -p branchless-lsp
  -> 63 + 5 + 6 + 3 = 77 passed, 0 failed, exit 0.
- Real-wire witness (BL9 client against the real server binary on real
  stdio, /tmp/wg2-wire/{branchy,clean,residual}.rs):
  - n_refused: 3 (branchy.rs, ERROR severity, code ANDON-REFUSED-001,
    at 0-based line 3 = the while line, char 10 — DWARF-mapped);
  - clean.rs -> 0 diagnostics; residual.rs -> 0 diagnostics;
  - max/conformanceVector (live): branchy -> refused=[ANDON_REFUSED],
    score 0.0; clean/residual -> admitted=[BRANCHLESS_ADMITTED],
    score 100.0. The vector flips — F6's conformance:False blocker is
    closed.
- F6 gate flip court (/Users/sac/scratch-wg2/repo, home-dir scratch per
  F6 precedent; topology guard refuses /tmp git init):
  - branchy staged -> ANDON_REFUSAL: refused 3 site(s), exit 1 (THE
    flip F6 recorded as BLOCKED — now fires against the real binary);
  - clean staged -> "1 staged .rs file(s) clean", proof written, exit 0.

## Falsifier / anti-vacuity

- refusal_flips_off_when_allowance_added: same while-loop body, +/- the
  allowance attr, flips refusal <-> residual (a court that cannot flip
  is FAIL).
- Gate flip court exits differ (1 vs 0) on branchy vs clean input.

## Divergences

- dispatch said "run compiler_worker compile (debuginfo per E9)" — used
  compile_debuginfo, as dispatched.
- Mach-O leading-underscore symbols and rustc's E0438-class refusal of
  unknown attrs were both load-bearing discoveries this session; both
  are pinned by tests.
- WG3's parameterized-attribute lane landed attributes.rs mid-session;
  parse_attrs/classify_seeded consumed as-found (no conflicts; one
  transient mid-edit failure of WG3's own test, resolved by WG3).
