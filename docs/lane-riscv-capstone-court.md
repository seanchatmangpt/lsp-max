# Lane RISCV receipt — riscv32imc capstone court leg

Closes BL1's UNVERIFIED: "riscv32imc capstone path compiles but was not
court-tested (no target installed check run)". Date: 2026-11-02.

## 1. Target state (was unknown, now verified this session)

- Active toolchain: `nightly-2026-04-15` (pinned via `rust-toolchain.toml`,
  repo root). `rustup target list --installed --toolchain nightly-2026-04-15`
  BEFORE: aarch64-apple-darwin, wasm32-unknown-unknown, wasm32-wasip1.
- **riscv32imc-unknown-none-elf was NOT installed** (BL1's suspicion was
  correct). Fix: `rustup target add riscv32imc-unknown-none-elf
  --toolchain nightly-2026-04-15` → "downloading component rust-std" →
  installed; the AFTER list shows `riscv32imc-unknown-none-elf`.
- Hand probe compile (exit 0): `rustc --crate-type=lib --emit=obj -O
  --target riscv32imc-unknown-none-elf` → `probe.o: ELF 32-bit LSB
  relocatable, UCB RISC-V, RVC, soft-float ABI` (file(1) witness; RVC flag
  set in the ELF header).

## 2. Defects found and fixed in the court module

`crates/branchless-lsp/src/disasm_court.rs` (court module named in-scope):

1. **ELF per-function `.text.<fn>` sections were invisible.** The court
   looked up only `__text`/`.text`; rustc ELF emits one section per
   `#[no_mangle]` fn, so `judge` saw zero functions — a vacuous court
   (witnessed live: "court saw no functions in riscv32imc object
   (vacuous)" panic).
2. **RISCVC mode was never enabled.** `.riscv().build()` defaults to
   RiscV32 with EMPTY extra_mode (capstone-0.12.0 src/arch/mod.rs:292-299,
   read this session). Now `mode(RiscV32).extra_mode([RiscVC])`.
3. **`c.jr` (the `ret` encoding) counted as a branch.** rf_free
   (`x.wrapping_add(1)`) = `c.addi; c.jr` → flagged 1 branch. `c.jr ra` is
   the canonical return, like arm64 `ret` (which the arm64 classifier
   already excludes); excluded in the RISC-V classifier + unit-test rows.
4. **Relocation placeholders aborted whole-section decode.** `disasm_all`
   returns Err on the illegal halfwords rustc leaves at relocation sites
   (witnessed: `.text.rf_branchy` mnemonics vanished wholesale). Fixed via
   `set_skipdata(true)` (`capstone_skipdata()` helper); `.byte` data insns
   never count as branches.

## 3. Court grid (tests/riscv32imc_court.rs)

| # | Court | Asserts | Status |
|---|---|---|---|
| 1 | `riscv32imc_branchy_flags_branchfree_passes` | rf_branchy branch_count > 0 (beqz/bnez class); rf_free == 0 | PASS |
| 2 | `riscv32imc_rvc_compressed_mnemonics_observed` | C-extension-only encodings (`c.addi4spn`, `c.lwsp`, `c.swsp`, `c.jr`) decoded — impossible without RISCVC mode; branch mnemonics (`bgeu`/`bltu`) seen | PASS |
| 3 | `riscv32imc_opcode_mutation_flips_the_court` | baseline 3 > 0; mutated /tmp copy flips to 0; `assert_ne` | PASS |

Note on Court 2: at `-O`, rustc emits the loop's conditional branches
UNCOMPRESSED (`bgeu`/`bltu`), not `c.beqz`/`c.bnez`. The court therefore
asserts the two facts separately: (a) compressed decodability via
C-extension-only encodings, (b) branch observation via the branch-mnemonic
filter. rustc does not let you force compressed branches, so a
`c.bnez`-specific wire assert would be a fake.

## 4. Flip witness (anti-vacuity)

- Court 3 zeroes the ENTIRE `.text.rf_branchy` section in a /tmp mutation
  copy of the probe object (never the shared tree): verdict flips
  branch_count 3 → 0, `assert_ne` holds.
- Two weaker mutations were tried and REJECTED as too weak, recorded as
  negatives:
  (a) single 0xe3 opcode byte → 3 → 2 (the second branch still counted);
  (b) byte-scoped zeroing of 0x63/0xe3 bytes → 3 → 1 (skipdata re-sync
  mints spurious branches from misaligned residue).

## 5. Gate

`cargo test -p branchless-lsp` → exit 0: 29 unit (lib) + 3 new riscv
courts + 0 (bin) = 32 passed, 0 failed, this session.

## 6. Divergences from dispatch

- Dispatch said "21 existing" tests; actual baseline is 29 unit tests in
  the lib target. New count: 32 total (29 + 3).
- `crates/branchless-lsp/tests/` did not exist; created it with the court
  file (dispatch allowed "extend or the court module").
- Scratch diagnostics used a temporary `examples/section_dump.rs`; deleted
  after use (not lane-owned).
- Dispatch named receipt `docs/lane-riscv-capstone-court.md` — written.
