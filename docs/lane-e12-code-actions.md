# Lane E12 Receipt: CodeAction mechanical rewriters

Date: 2026-11-02 (session). Repo: /Users/sac/lsp-max (branch main).
Standing: **ALIVE**.

## Files written (lane ownership)

- `crates/branchless-lsp/src/code_actions.rs` — placeholder replaced by 3
  real WorkspaceEdit producers (`TextEdit` 0-based line/char coords, end
  exclusive; `apply_edit` for round-trips).
- `crates/branchless-lsp/tests/code_actions_courts.rs` — new integration
  courts (round-trip + anti-vacuity).
- `docs/lane-e12-code-actions.md` — this receipt.

## Gate (run this session)

- `CARGO_TARGET_DIR=/tmp/e12-target cargo test -p branchless-lsp` → exit 0:
  43 lib tests, 5 code_actions_courts, 3 riscv32imc_court, 2 main bins
  (0+0). All green; no skips.

## Rewriter 1: if/else -> mask math

Before:
```rust
fn f(cond: bool, x: u32, y: u32) -> u32 {
    if cond { x } else { y }
}
```
After (`apply_edit` output, Tier-1 clean, parses, planted-probe verified):
```rust
fn f(cond: bool, x: u32, y: u32) -> u32 {
    { let mask = ((cond) as usize).wrapping_neg(); (({ x }) & mask) | (({ y }) & !mask) }
}
```
Refusals: `NoFix::IfWithoutElse` (both arms required);
`NoFix::NoRewritableExprAtLoc`. Edit carries the Tier-2-on-save note.

## Rewriter 2: match -> jump table

Before:
```rust
enum Op { A, B, C }
fn g(op: Op, v: u32) -> u32 {
    match op {
        Op::A => 10, Op::B => 20, Op::C => 30,
    }
}
```
After (table inserted at file top, match replaced):
```rust
#[allow_isel_residual]
const OP_JUMP_TABLE: [Op; 3] = [10, 20, 30];
enum Op { A, B, C }
fn g(op: Op, v: u32) -> u32 {
    { let __idx = (op) as usize; unsafe { *OP_JUMP_TABLE.get_unchecked(__idx) } }
}
```
Flatness proof: all variant discriminants (explicit or implicit) must be
exactly contiguous 0..=n, arms one-to-one with variants, no guards,
or-patterns, struct/range patterns. Non-flat enums (e.g. `enum Weird { X = 5, Y = 9 }`)
get a documented `NoFix::NonFlatEnum` refusal naming the reason — never a
wrong rewrite. FINDING: the first implementation emitted `TABLE[idx]`
(ExprIndex), which the crate's own Tier-1 linter flags as
ANDON_BRANCH_SYNTAX — the rewrite now emits `get_unchecked` (sound: the
scrutinee's discriminants were proven 0..=n, so the cast is in bounds).

## Rewriter 3: allow_isel_residual insertion

Inserts `#[allow_isel_residual(target = "riscv32", max_branches = N,
reason = "measured")]` above the target fn. `N` = the Tier-2 court's
measured branch_count when the LSP has one; unmeasured functions get
`reason = "unmeasured"` with `max_branches` honestly omitted.

## Anti-vacuity (court flips)

`court_if_else_mask_semantics_flip_on_inverted_mask`: swaps mask polarity
in the emitted text; asserts the mutated text fails the canonical-shape
check AND that the arithmetic itself flips (mask picks then-branch 10,
inverted picks 20). Court cannot pass on the wrong rewrite.

## Divergences / disclosed items

1. Edit coords are 0-based line + char (not LSP UTF-16); self-consistent
   via apply_edit round-trips. Documented in code_actions.rs docs.
2. Minimal unblock under the compile-freeze SLA: added one closing brace
   in `disasm_court.rs judge_classified` while its owner lane was
   mid-write; the owner lane repaired the region itself within the SLA
   window and their repair superseded mine. Both directions disclosed.
3. `attributes::parse_attrs` refuses the parameterized
   `#[allow_isel_residual(target=..., ...)]` form this rewriter emits
   (unknown-attr error path). Not my file; needs the attributes lane to
   accept the parameterized form. Recorded, not fixed.
4. Anti-vacuity mutation semantics are structural + arithmetic (in-crate
   eval of the emitted text), not a rustc compile of the rewritten
   snippet; a compile-based court would need the compiler worker and is
   listed as future work.
5. test-root isolation: cargo build roots leased at /tmp/e12-target
   (deleted at lane end).
