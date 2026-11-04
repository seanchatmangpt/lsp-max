# Lane WG3 Receipt — parameterized `#[allow_isel_residual]` parser

- Lane: WG3 (lsp-max), 2026-11-02
- Repo: /Users/sac/lsp-max (canonical checkout, branch main)
- Subject: `crates/branchless-lsp/src/attributes.rs` (sha256
  `686ecd596745dc7f9fbc826208277fc6b09d623d53e8a19dfb6431d88f9a024e`)
- E12 gap closed: `parse_attrs` now accepts
  `#[allow_isel_residual(target = "...", max_branches = N, reason = "...")]`,
  so E12's `insert_allow_isel_residual` output is no longer dead.

## Grammar (parse_attrs, WG3 extension)

- Bare forms unchanged: `allow_isel_residual`, `branchless_invariant`.
- Parameterized form: `allow_isel_residual(` `key = value` `,` ... `)`
  with keys `target` (quoted str), `max_branches` (usize literal),
  `reason` (quoted str).
- Typed refusals (`AttrParseErrorKind`):
  `UnknownParamKey` (unknown key, incl. parameterized
  `branchless_invariant(...)`), `MalformedParam` (bad usize / unquoted
  str / empty segment), `DuplicateParam`, `MissingCloseParen`,
  plus the pre-existing `UnknownAttr`.
- The parameterized form does NOT set the bare `allow_isel_residual`
  flag — the allowance stays target-scoped, so mismatched-target
  fallback keeps bare refusing semantics.

## Classification semantics

- New `ResidualParams` (target / max_branches / reason) with
  `target_matches` (exact, `any`, or target-prefix: `riscv32` covers
  `riscv32imc`), `classify_count` (0 → Admitted; ≤ max → HintResidual;
  \> max → ErrorRefused), `reason_note()` provenance string.
- `ResidualGrid::classify_seeded_count` (new): parameterized allowance
  supersedes the bare count semantics for its symbol when the target
  matches; bare-form behavior unchanged (court grid:
  `residual_at_exactly_max_branches_refused_at_max_plus_one`).
- `classify` (bool) honors residual_params conservatively (budget ≥ 1 →
  HINT; budget 0 → refusal); exact-count courts use the count forms.
- Reason carried via `ResidualParams::reason_note()` — the literal
  reason plus budget, formatted for diagnostics + receipt provenance.
  Git-blame provenance: future work (not available in-process today;
  consumers must call `reason_note()` from their surfaces).

## Courts (new, in attributes.rs tests)

parameterized_attr_parses_full_payload ·
unmeasured_parameterized_form_has_no_budget ·
unknown_param_key_is_typed_refused ·
malformed_and_duplicate_params_are_typed_refused ·
parameterized_branchless_invariant_is_typed_refused ·
residual_at_exactly_max_branches_refused_at_max_plus_one ·
parameterized_allowance_is_target_scoped ·
reason_surfaces_in_provenance_note ·
bare_form_behavior_unchanged_under_parameterized_extension ·
classify_count_flips_across_budget_boundary

## Gate (this session)

- Baseline pre-edit: `cargo test -p branchless-lsp` → exit 0.
- Final: `CARGO_TARGET_DIR=/tmp/wg3-target cargo test -p branchless-lsp`
  → exit 0 TWICE (63 lib + 5 + 6 + 3 integration, all green), including
  the seeded grid + E12 code_actions courts.

## Anti-vacuity flip (SHIM-COURT)

- Mutant: ignore `max_branches` (`b if Some(b) <= max` arm deleted →
  always HintResidual). Applied in the shared tree under /tmp backup,
  courts run: 3 WG3 courts FAILED, exit 101
  (classify_count_flips_across_budget_boundary,
  residual_at_exactly_max_branches_refused_at_max_plus_one,
  parameterized_allowance_is_target_scoped).
- Restore: sha256-exact
  `686ecd596745dc7f9fbc826208277fc6b09d623d53e8a19dfb6431d88f9a024e`
  + mtime bump (`touch`) per the mtime-restore law; post-restore gate
  green.
- A court that cannot flip is FAIL; this court flips.

## Divergences / notes

- OWNERS said "attributes.rs + tests only"; WG2 landed `rule_pack.rs`
  mid-lane, whose `run_tier2` surfaces needed no WG3 edits. During the
  lane, WG2's in-flight edits twice broke the shared compile (missing
  `lsp_max` crate path; a mid-write unterminated string in rule_pack.rs)
  — both recovered by WG2 within the SLA; no WG3 unblock was applied
  there.
- Minimal disclosed unblock (compile-freeze SLA, disclose-in-both):
  `crates/branchless-lsp/src/receipts.rs` tests (2 struct literals)
  gained `..Default::default()` so the new `residual_params` field
  compiles. No behavior change.
- Two of WG2's new rule_pack tests were flaky during overlap (pass
  isolated, fail in full runs) — settled green in the final two
  consecutive gate runs; the flake belongs to WG2's lane, recorded here
  as witnessed, not repaired by WG3.
- One pre-existing flake witnessed in
  `disasm_court::tests::branchfree_host_object_is_admitted` (real `cc`
  subprocess; failed once in a batch run, passed on rerun).
