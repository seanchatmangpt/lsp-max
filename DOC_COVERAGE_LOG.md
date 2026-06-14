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

---

## Iteration 2 — 2026-06-14 · commit d3cb8d0 (clean tree)

### Triple closed: `Receipt` (BLAKE3 content-addressing + Merkle chain)

- **doc** — `lsp-max-protocol/src/core.rs` rustdoc on `Receipt` now references the
  example; the example keeps its Diataxis explanation of why hash-the-artifact beats
  trust-the-assertion.
- **example** — `examples/receipt_chain_explained.rs`: was prose-only (printed a
  pointer), now a real witness using the actual `Receipt` struct + `blake3`
  (root dev-dep, same hash `anti-llm-cheat-lsp/src/ocel.rs` uses) + `tempfile`.
  5 assertions: content-addressing verifies, tamper is detected, the circular-hash
  trap is detectable, genesis has no prev hash, the chain link survives serde.
- **link** — doc→example (rustdoc) and example→doc (`core.rs` / `ocel.rs`).
- **captured run** (`cargo run --example receipt_chain_explained`, real `$? = 0`):
  ```
  WITNESS receipt_chain: 5 contract assertions held
    [1] receipt from final bytes verifies against the file
    [2] modifying the artifact makes the receipt fail to verify (tamper-evident)
    [3] the circular-hash trap is detectable (digest != final file)
    [4] genesis has no prev hash; the next receipt links the prior hash
    [5] serde roundtrip preserves the chain link
  ```

### Step-7 finding (doc described behavior the *type* lacks)
The old `receipt_chain_explained` and the doctrine describe BLAKE3 hashing/verification,
but the `Receipt` *type* (`core.rs`) is a bare data carrier — no hash/verify method.
The hashing lives in `anti-llm-cheat-lsp/src/ocel.rs` (`write_ocel_outputs`) and chain
verification in `lsp-max-runtime/src/ledger.rs` (`verify_instance_ledger`, sha256, LSP_1
conventions). The witness therefore demonstrates the doctrine *pattern* with the real
`Receipt` struct as carrier, and points to those production sites — it does not pretend
the type self-verifies.

### Updated gap map (run-to-exit single-file examples)
| Example | Status |
|---|---|
| `repro_lifecycle.rs` | ✅ covered |
| `conformance_vector_explained.rs` | ✅ covered (iter 1) |
| `receipt_chain_explained.rs` | ✅ covered (iter 2) |
| `calver_law_explained.rs` | ❌ doc-without-example (queued) |
| `custom_notification.rs` | ⚠ unclassified (exit 124 — server-style?) |
| `stdio.rs` / `tcp.rs` / `websocket.rs` | ⊘ server-class (witnessed by tests/) |

### Out-of-loop finding (reported, not chased)
`tower-lsp-max-runtime/` is **tracked in this repo** (`src/lib.rs`,
`refund_receipt.txt`) — the directory name embeds "tower-lsp", which AGENTS.md law #1
forbids outside negative-control fixtures. `lsp-max-runtime/` is the live runtime
crate (dep of the root); `tower-lsp-max-runtime/` appears to be a stale duplicate.
Flag for the maintainer — not a doc-loop change.

### Hard stops
None.

---

## Iteration 3 — 2026-06-14 · commit 7e8e235 (clean tree)

### Triple closed: CalVer version law (`ANTI-LLM-VERSION-*`)

- **doc** — `examples/anti-llm-cheat-lsp/src/rules/version.rs` (the production
  enforcement) now has a module doc citing the example; the example keeps its
  Diataxis explanation of why CalVer, not SemVer.
- **example** — `examples/calver_law_explained.rs`: was prose-only, now validates
  the crate's live `env!("CARGO_PKG_VERSION")` against the YY.M.D law and rejects
  SemVer-shaped/malformed strings (1.2.3, 26.13.1, 26.6.32, 26.6, v26.6.9, -rc1).
  Load-bearing: if the workspace is ever bumped to non-CalVer, the example panics.
- **link** — doc→example (version.rs module doc) and example→doc (header cites
  Cargo.toml + version.rs).
- **captured run** (`cargo run --example calver_law_explained`, real `$? = 0`):
  ```
  WITNESS calver_law: version law holds for this crate
    actual CARGO_PKG_VERSION = 26.6.9 (valid YY.M.D)
    [1] this crate's real version is lawful CalVer
    [2] release-date-shaped versions accepted (26.6.9, 24.1.1, 26.12.31)
    [3] SemVer/malformed rejected (1.2.3, 26.13.1, 26.6.32, 26.6, v.., -rc1)
  ```

### `custom_notification` classified → ⊘ server-class
Confirmed server-style: builds `Server::new(stdin, stdout, socket).serve(...)` on
`tokio::io::stdin/stdout` and blocks (the earlier exit 124 was the block, not a
hang-bug). Witnessed by the transport/integration tests, not run-to-exit.

### Gap map — run-to-exit single-file examples (BIJECTIVE for this scope)
| Example | Status |
|---|---|
| `repro_lifecycle.rs` | ✅ covered |
| `conformance_vector_explained.rs` | ✅ covered (iter 1) |
| `receipt_chain_explained.rs` | ✅ covered (iter 2) |
| `calver_law_explained.rs` | ✅ covered (iter 3) |
| `stdio.rs` / `tcp.rs` / `websocket.rs` / `custom_notification.rs` | ⊘ server-class (witnessed by tests/) |

**documented-but-unexercised: 0 · exercised-but-undocumented: 0** for the single-file
run-to-exit scope. Every run-to-exit demo is now a real witness or a classified server.

### Next frontier (scope expansion, not prose padding)
1. **Cross-product example** (the goal's coherence test): `ConformanceVector` +
   `Receipt` composing — receipt verification moves the Receipt axis out of
   `unknown`, then the gate admits release. No single-API example shows this.
2. **Broader documented surface**: the loop has covered the 8 single-file examples;
   the root crate's full `///`-over-`pub` API (e.g. `LspService`, `Server`,
   `ComposedServer`, gate primitives) is a larger documented surface whose
   example-coverage is not yet mapped. Next iterations enumerate that surface.

### Hard stops
None.
