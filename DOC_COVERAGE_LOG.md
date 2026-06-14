# DOC_COVERAGE_LOG

Bijective doc↔example coverage for the **root `lsp-max` crate's run-to-exit
examples**. A capability is `✅ covered` only when a doc describes it, an example
in `examples/` exercises it, the example **ran in the cited iteration** (real exit
code captured), and the example asserts the contract so it breaks if the capability
is fake. Prose alone is never coverage.

**Scope of this loop:** the 8 single-file `cargo run --example <name>` targets of
the root crate (the run-to-exit demos). The 11 example *crates*
(`anti-llm-cheat-lsp`, `pattern-lsp`, `wasm4pm-lsp`, …) are LSP servers that block
on stdio — they cannot run-to-exit and are witnessed by their dogfood test suites,
not by this loop. Runner: `cargo run --example <name>`. Toolchain: cargo 1.97.0-nightly.

---

## Iteration 1 — 2026-06-14 · commit 3f96b29 (clean tree)

### Gap map — run-to-exit single-file examples

| Example | Capability | Exercises it? | Ran (exit) | Status |
|---|---|---|---|---|
| `repro_lifecycle.rs` | `max/snapshot` over `LspService`/`Server` duplex | YES — builds service, sends real request | 0 | ✅ covered |
| `conformance_vector_explained.rs` | `ConformanceVector` 3-valued law (Unknown ≠ Admitted/Refused) | YES — 5 contract `assert!`s (this iteration) | 0 | ✅ covered |
| `calver_law_explained.rs` | CalVer version law (`ANTI-LLM-VERSION-*`) | NO — `main()` only `println!`s a pointer | 0 (meaningless) | ❌ doc-without-example |
| `receipt_chain_explained.rs` | BLAKE3 `Receipt` content-addressing | NO — `main()` only `println!`s a pointer | 0 (meaningless) | ❌ doc-without-example |
| `custom_notification.rs` | custom LSP notification surface | unclassified — blocks (exit 124, server-style?) | 124 | ⚠ classify next |
| `stdio.rs` / `tcp.rs` / `websocket.rs` | transport servers | server-class (block by design) | n/a | ⊘ witnessed by `tests/`, not run-to-exit |

**Key finding:** three "*_explained" examples were **doc-laundering** — their `main()`
prints a pointer to other files and exits 0, so a passing `cargo run` witnessed
nothing (the documentation form of a benchmark reporting `0 measured`). The prose is
accurate Diataxis "Explanation"; the failure is that nothing *ran* the capability.

- documented-but-unexercised: `calver_law_explained`, `receipt_chain_explained`
  (and `conformance_vector_explained` until this iteration closed it)
- exercised-but-undocumented: none found in the single-file set

### Triple closed this iteration: `ConformanceVector`

- **doc** — `lsp-max-protocol/src/conformance.rs` rustdoc on `ConformanceVector` now
  references the example as the runnable witness; the example keeps its accurate
  Diataxis explanation of *why* Unknown must not collapse.
- **example** — `examples/conformance_vector_explained.rs`: real `main()` constructs
  `ConformanceVector`s and asserts the contract (5 assertions), incl. the load-bearing
  law — an unknown axis is not admitted and blocks release under strict mode, and the
  `set_unknown`→`set_admitted` transition keeps the three sets disjoint. Panics if the
  law regresses.
- **link** — doc→example (rustdoc) and example→doc (header points to
  `conformance.rs` / `src/gate.rs`).
- **captured run** (`cargo run --example conformance_vector_explained`, real exit
  `$? = 0`):
  ```
  WITNESS conformance_vector: 5 contract assertions held
    [1] all-admitted vector admits release
    [2] unknown axis is NOT admitted and BLOCKS release under strict mode
    [3] non-strict tolerates unknown for release but never counts it admitted
    [4] refused axis blocks release in any mode (distinct from unknown)
    [5] set_unknown→set_admitted keeps the three axis sets disjoint
  ```
  Demonstrated: replacing the assertions with the optimistic-collapse behavior the
  doc warns against would flip assertions [2]/[3] and the example would exit non-zero.

### Queued for review (not batch-committed)
- `calver_law_explained` → real witness: construct/validate a CalVer version and
  assert a non-conforming version is rejected (find the version-law check first).
- `receipt_chain_explained` → real witness: hash an artifact with BLAKE3, write the
  `Receipt`, re-hash, `assert!` digest matches; demonstrate the circular-hash trap
  failing verification. Needs `Receipt` API in `lsp-max-protocol/src/core.rs` + file I/O.
- `custom_notification` → classify: server-class (move to ⊘) or a run-to-exit demo
  that currently hangs (a real finding).

### Hard stops
None.

### Cross-product candidates (after per-capability coverage)
- `ConformanceVector` + `Receipt` + gate: an end-to-end example where receipt
  verification moves the `Receipt` axis out of `unknown` and the gate then admits
  release — shows the admission model *composing*, not just each piece in isolation.
