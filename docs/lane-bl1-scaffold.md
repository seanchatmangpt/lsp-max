# Lane BL1 receipt — branchless-lsp scaffold

Repo: /Users/sac/lsp-max (canonical checkout, branch default). Date: 2026-11-02.

## Files written (all under crates/branchless-lsp/ plus one disclosed root edit)

- `crates/branchless-lsp/Cargo.toml` (new)
- `crates/branchless-lsp/src/lib.rs` — crate docs (two-tier verification), 7 pub
  modules, `BranchlessLspServer` implementing the real
  `lsp_max::rule_pack_server::RulePackServer` + `LanguageServer` lifecycle
  (initialize/shutdown/didOpen/didChange/didSave).
- `src/ast_linter.rs` — syn::visit Tier 1 pass (ExprIf/Match/ForLoop/While/
  Loop/Try/Index/unwrap/expect/panic-macros → ANDON_BRANCH_SYNTAX hits with
  1-based line/col via proc-macro2 span-locations).
- `src/compiler_worker.rs` — debounced `rustc --crate-type=lib --emit=obj -O`
  driver, aarch64-apple-darwin / riscv32imc-unknown-none-elf, /tmp staging
  with per-source BLAKE3-keyed filenames (concurrent-compile race fix).
- `src/disasm_court.rs` — Tier 2 court: object::File parse, .text/__text
  slicing, gap-derived function extents (Mach-O symbol sizes are 0), capstone
  ARM64 + RISC-V(32+C) mnemonic classification, COURT-SKIP loud skips.
- `src/attributes.rs` — allow_isel_residual / branchless_invariant parsers +
  classify() HINT-residual vs ERROR-refused matrix.
- `src/hover.rs` — hover card composing court + lint + attrs.
- `src/code_actions.rs` — 3 quick-fix families.
- `src/receipts.rs` — 3-vector ⟨N_admitted, N_refused, N_residual⟩, BLAKE3
  receipts to `.cgap/receipts/branchless/`.
- `src/main.rs` — stdio JSON-RPC bootstrap (`LspService` + `Server`).
- Root `Cargo.toml`: ONE disclosed edit — workspace members +=
  `"crates/branchless-lsp"` (after `crates/ra-max`).
- This receipt.

## RulePackServer existence finding

`RulePackServer` EXISTS in lsp-max: `src/rule_pack_server.rs:691`
(required methods: rule_packs, grammar, server_name, client, adapter).
BranchlessLspServer implements the real trait — no local re-definition.
Deps divergence from dispatch: `lsp-max = { path = "../.." }` (house
convention, ra-max-style; dispatch's `../lsp-max` would not resolve from
crates/branchless-lsp), and `lsp-types-max` (workspace) instead of
`lsp-types 0.95` — the RulePackServer surface's types are lsp-types-max.

## Gates (this session, exact commands)

- `cargo check -p branchless-lsp` → exit 0
- `cargo test -p branchless-lsp` → exit 0, **21 passed, 0 failed**
  (all with CARGO_TARGET_DIR=/tmp/bl1-target)
- `cargo clippy -p branchless-lsp --all-targets -- -D warnings` → exit 0

## Anti-vacuity (court flip evidence, same run)

- `disasm_court::tests::mnemonic_classification_flips` — boundary mnemonics
  flip is_branch_mnemonic for both ISAs.
- `disasm_court::tests::branchy_host_object_is_flagged` — real rustc
  --emit=obj of a black_box while-loop yields branch_count > 0 (b.hs-class);
  branchfree probe yields 0. An if/else probe compiled to a branchless cmov
  at -O (retired probe source) — Tier 2 reads the real machine code, not
  the source.
- `compiler_worker::tests::compiles_host_object_and_lints_branchfree_code` —
  real rustc run, object file asserted to exist.
- `receipts::tests::hash_flips_when_vector_changes` — vector mutation flips
  the BLAKE3 receipt hash.

## Documented scaffold limits / divergences

1. Tier 2 line-info: gimli PC→line mapping is plumbed in docs but not yet
   executed in judge() (fnVerdict has no PC→line field yet); gimli dep is
   present. Future lane work.
2. Whole-object ParsedAttrs in ReceiptVector::fold (real surface is
   per-function attrs) — asserted with the (1,0,2) whole-object semantics.
3. did_save has no RulePackServer default handler; it is a typed no-op in
   the lifecycle impl.
4. riscv32imc capstone path compiles but is court-untested this session
   (no riscv32 target installed check was run) — UNVERIFIED, not PASS.
