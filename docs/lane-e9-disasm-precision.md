# Lane E9: disasm_court precision lift

Subject: `crates/branchless-lsp/src/disasm_court.rs`,
`crates/branchless-lsp/tests/e9_precision_court.rs`,
`crates/branchless-lsp/src/compiler_worker.rs` (one disclosed addition).

## 1. DWARF mapping executed (BL1 handoff closed)

BL1 recorded the DWARF PC->line mapping as "plumbed but not executed".
E9 executes it: `CompilerWorker::compile_debuginfo` compiles probes with
`-g --emit=obj`; `LineMap::parse` walks the object's `.debug_line` /
`__debug_line` section unit-by-unit (32/64-bit DWARF initial length) via
gimli 0.31 and collects (pc, line, column) rows; `judge_classified`
attaches line + column to every classified branch site.

Mapping witness (court 1, host aarch64, this session):
`e9_dwarf_line_correlation_lands_inside_function_span` — every branch site
of the loop probe resolves to a line inside the function span, and the
back-edge pins the `while` source line EXACTLY (line 5, column 11). This is
line-level correlation inside the span, not symbol-level.

## 2. Classification courts

Stated classification rule (mechanical, on the real machine code):

- Conditional branch whose resolved target is strictly below its own pc:
  `LoopStructural` (back-edge).
- Forward conditional edge whose target lands immediately past a back-edge
  (target == back_edge.addr + back_edge.size): loop-EXIT edge, also
  `LoopStructural`. Witnessed necessity: a compiled while loop at -O has a
  forward exit edge (riscv `bgeu ... 0x14` resolving past the loop) and an
  arm64 loop (witnessed layout: `b.hs #0x48` exit at 0x28 pairing with the
  conditional back-edge `b.lo #0x30` at 0x44). Both loop shapes are covered
  by the two rules above.
- `bl`/`jal` -> `Call`; `b`/`br`/`j`/`c.j`/`jalr`/`ret` -> `Unconditional`;
  unparseable target -> `Conditional` (fail-closed refusal).

Dual-mode proof (court 2): crafted byte sequences decode and classify under
the RiscV32+RISCVC capstone handle: 32-bit backward `beq` (63 8C 00 FE) ->
LoopStructural; 32-bit forward `beq` (63 88 00 00) -> Conditional;
compressed backward `c.beqz` (79 DC) -> LoopStructural; compressed forward
`c.bnez` (99 EC) -> Conditional; `c.j` (21 A0) -> Unconditional. All five
encodings verified against capstone decode before being pinned.

No-false-positive proof (court 3): the branchfree fn yields ZERO branch
sites (prologue/epilogue and loop-counter increments produce none), and the
counted loop is admitted with andon_refused == false.

## 3. Column precision

The active rustc (nightly 1.101.0, aarch64 host) DOES emit DWARF column
data for `-g --emit=obj`; court 4 asserts BranchSite.column surfaces it.
No line-level floor was required this session; the [SKIP-floor] path
remains for toolchains that emit none.

## 4. Anti-vacuity (courts 5 + 6, one per arch)

Mutations run on /tmp copies; the shared tree is untouched.

- riscv32imc: zeroing every LoopStructural site's bytes in
  `.text.e9_loop` flips the verdict — after mutation no LoopStructural
  classification remains (assert_ne on the class vectors).
- arm64: same mutation discipline on `__text`.

A subtlety the court caught: zeroing only the FIRST LoopStructural site
leaves its paired exit edge intact and the verdict does not flip — the
courts zero the whole equivalence class.

## 5. Commands + exits (this session)

- `cargo build -p branchless-lsp` (CARGO_TARGET_DIR=/tmp/e9-target): exit 0.
- `cargo test -p branchless-lsp`: EXIT=0 — unit tests (E10 four-way class
  courts included), e9_precision_court 6/6, riscv32imc_court 3/3,
  doc-tests 0. The riscv32imc lane's courts still pass on the extended
  file (extended, not duplicated).

## 6. Divergences + disclosures

- compiler_worker.rs is outside the stated OWNERS list; the dispatch item
  "compile a probe via compiler_worker WITH debuginfo" required a
  `compile_debuginfo` entry point, which E9 added (disclosed here).
  `compile_debuginfo` also keys staging UNIQUELY per call: parallel courts
  on identical probe sources raced on one shared .o path and tore each
  other's object (witnessed this session); the per-call atomic suffix
  fixes that race.
- The file grew mid-lane from concurrent lane E10 (four-way InsnClass).
  E9 extended on top of E10's state rather than duplicating.
- A reconstruction event: a lane-side scripting error truncated
  disasm_court.rs mid-edit; the full module was rebuilt from the lane
  transcript and the crate re-verified green afterwards. Final on-disk
  state is the tested state.
- Target semantics pinned empirically (probe runs this session): arm64
  prints ABSOLUTE targets (`b.lo #0x28`); riscv32c prints RELATIVE offsets
  (`bltu a2, a3, -0xa`, `bgeu a2, a1, 0x14` resolving past the loop).

Standing: ALIVE for the six courts + full crate gate above. Perf: none
claimed (no benchmark run).
