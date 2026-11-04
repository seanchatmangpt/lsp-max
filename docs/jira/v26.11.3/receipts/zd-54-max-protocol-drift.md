# ZD-54 — max-protocol.ttl Notification drift from src/andon/lsp.rs

Date: 2026-11-04 · Repo: lsp-max @ 6673823 (post ZD-47 push)

## Authority ruling

`src/andon/lsp.rs`'s `Notification::METHOD` associated consts are the **code authority**.
`ontology/max-protocol.ttl` is **hand-law** (not generated): a generator grep found only
consumers — `crates/lsp-max-cli/src/nouns/ontology.rs` (concatenates the ttl) and
`crates/lsp-max-cli/src/gen/testmatrix_generator.rs` (reads it). Nothing writes it.
Therefore the **ontology was the stale side** and was fixed to mirror the code authority;
the tests' expectations were updated only as a consequence of that fix (5 real consts,
not an arbitrary agreement).

## The drift

`max-protocol.ttl` declared 7 `lspMax/*` `lsp:Notification` individuals. Five mirror real
consts (`andonRaised`, `admissionChanged`, `truthTableChanged`, `counterfactualFailed`,
`nextLawfulStepChanged`). Two — `lspMax_receiptArrived` and `lspMax_gateChanged` — had
**zero Rust implementation** (grep across `src/` and `crates/`: no hits for
`receiptArrived`/`gateChanged`/`ReceiptArrived`/`GateChanged`). They were CP10-era
aspirational entries kept only "for naming consistency", directly contradicting the
fidelity proof's exact-mirror law. This made both proof tests red:

- `max_protocol_gates_and_sparql_derived_proof.rs:86` — live COUNT returned 7 vs asserted 3.
- `max_protocol_notification_fidelity_proof.rs:87` — declared set (7) ≠ expected (3).

## Fix (ontology side, authority-following)

- `ontology/max-protocol.ttl` — removed `lsp:lspMax_receiptArrived` and
  `lsp:lspMax_gateChanged` individuals; updated the CP10 header note with the ZD-54 ruling
  (re-add an individual only when its `const METHOD` exists).
- `tests/max_protocol_gates_and_sparql_derived_proof.rs` — COUNT assertion 3 → 5 (the
  hand-transcribed total now equals the real const count).
- `tests/max_protocol_notification_fidelity_proof.rs` — `EXPECTED_METHOD_NAMES` now the 5
  real consts from `src/andon/lsp.rs`; doc comment "three" → "five"; stale "restricted to
  the three this ontology file declares" note removed.

Both sides were not edited to agree arbitrarily: the ttl change is dictated by the code
authority, and the test expectations now transcribe that authority directly.

## Verification

```
cargo test --test max_protocol_gates_and_sparql_derived_proof \
            --test max_protocol_notification_fidelity_proof
run 1: 3 passed + 4 passed, 0 failed
run 2: 3 passed + 4 passed, 0 failed, exit 0
```

## ZD-47 follow-through (step 1)

- `crates/lsp-max-compositor/tests/diagnostic_relay_probe.rs` — uncommitted ZD-47 edit
  (resolve_child_bin + loud [SKIP]) verified intact on disk, committed as `6673823`
  (git commit -F) and pushed to origin/master (first attempt; no retry needed).
- `examples/wasm4pm-lsp/tests/receipts/cog010-scan.json` — stray modification confirmed
  to be a test side effect (regenerated `receipt_id` + `scanned_at` timestamp per run of
  `cog010_no_oracle_injection`). Regenerable → `git checkout --`, not committed. Disclosed
  here per lane law.
