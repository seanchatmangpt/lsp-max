# Lane E10 — conformance vector + receipt rigor lift

## Vector schema

`ReceiptVector` extended from 3 to 5 fields
(`crates/branchless-lsp/src/receipts.rs`):

```text
⟨N_admitted, N_refused, N_residual, N_alu, N_load_store⟩
```

- `n_alu` / `n_load_store` are instruction-class totals fed by
  `DisasmReport::total_alu()` / `total_load_store()`; `fold` and
  `fold_seeded` both populate them from the court report.
- Canonical form (zero-alloc serialized into `ReceiptScratch`, cap 512B,
  `RECEIPT_SCRATCH_CAP`):

```text
branchless-receipt admitted=N refused=N residual=N alu=N ls=N
```

## Instruction-class feed (disasm_court.rs)

`classify_insn(arch, mnemonic) -> InsnClass` — four-way classification
(Branch / Alu / LoadStore / Other), per-ISA mnemonic tables for ARM64 and
RISC-V 32C. `is_branch_mnemonic` is now `classify_insn == Branch`, so the
pre-existing branch-admission semantics are byte-for-byte preserved
(`ret` is NOT a branch for the branchless invariant; returns are not
admission-relevant). `DisasmCourt::count_classes` returns
`(branch, alu, load_store, other)` per code range; `FnVerdict` carries
`alu_count`, `load_store_count`, `other_count`, and the report exposes
`total_alu()` / `total_load_store()`.

## Receipt binding (Φ_lease)

```text
R = BLAKE3( "src"|len|SourceBytes ∥ "elf"|len|ElfTextBytes ∥
            "cfile"|len|C_file ∥ "lease"|Φ_lease ∥ "vec"|canonical )
```

Tagged length-framing prevents segment-boundary collisions. Φ_lease =
`session_lease()` — a thread-scoped BLAKE3-distilled process-start
entropy digest (address layout + instant + stack anchor). It is the LSP
SESSION identity, NOT the cgap P28 license lease; the two never conflate.
Stable within a thread for the session; new sessions differ.

Zero-allocation serialization: `ReceiptVector::serialize_into(&mut
ReceiptScratch)` emits bytes with manual digit emission (no `format!`
in the loop). Overflow past the cap is a typed refusal
(`ReceiptOverflow`, maps to `io::ErrorKind::FileTooLarge` on the bound
write path) — never truncation, never a grow.

## Court grid (all run this session)

| Court | File | What it kills |
|---|---|---|
| `four_way_classification_flips_every_boundary` | disasm_court.rs | mnemonic table drift (each class boundary flips) |
| `known_class_mix_object_court` | disasm_court.rs | class totals on a real compiled object with known ALU/LS/branch mix |
| `canonical_form_has_all_five_fields` | receipts.rs | 3-field canonical regression |
| `scratch_overflow_is_typed_refusal` | receipts.rs | silent truncation / silent growth |
| `alu_count_flip_changes_hash` | receipts.rs | anti-vacuity: ONE class-count flip MUST flip R |
| `bound_receipt_flips_on_any_segment` | receipts.rs | src/elf/cfile segment mutation flips R |
| `session_lease_is_stable_within_thread...` | receipts.rs | zero/duplicate lease |
| `fold_feeds_instruction_class_counts` | receipts.rs | vector not fed from court |
| `serialize_loop_is_zero_allocation` | receipts.rs | counting-allocator court: 0 allocs in serialize loop, with positive control (format! must register allocs — dead counter = vacuous court) |

## Alloc witness

`serialize_loop_is_zero_allocation`: counting `GlobalAlloc` counts
allocations on the measuring thread; serialize loop witnessed 0 allocs;
positive control (`format!` immediately after) witnessed > 0 — the zero
is measured, not vacuous.

## Gate

`cargo test -p branchless-lsp` — exit 0. 43 lib + 3 riscv32imc courts,
0 failures.

## Divergences / disclosures

- C_file is the companion C declaration/source bytes (may be empty);
  the dispatch did not specify a concrete producer, so `write_bound`
  takes it as caller-provided bytes.
- Compile-freeze SLA unblock fixes (disclosed in both lanes' receipts):
  - `code_actions.rs` (other lane, in-flight): added syn `printing`
    feature; fixed 4 corrupted `.to_token_stream()` expression sites and
    a `collect_enums` lifetime.
  - `disasm_court.rs` region I do not own-by-content (E9 line-map lane's
    in-flight `judge_classified`/`LineMap`/`BranchClass` code inside my
    owned file): repaired brace/impl structure, `ColumnType::LeftEdge`,
    NonZero compare, `program()` args. Structure only — no semantics
    changed beyond making it compile.
  - `hover.rs`: filled the 3 new `FnVerdict` fields with 0 in the
    pre-existing constructor.
- Φ_lease entropy is not cryptographic-grade at rest (process-start
  entropy distilled by BLAKE3); it is session identity, not a secret.
