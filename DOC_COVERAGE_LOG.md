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

---

## Iteration 4 — 2026-06-14 · commit 118b2b0 (clean tree)

### Cross-product closed: `Receipt` × `ConformanceVector` (composition)

The goal's coherence test — capabilities composing, not just each in isolation.
New example `examples/admission_pipeline.rs`: receipt verification is the *evidence*
that resolves the `Receipt` law axis, and the gate (`admits_release`) reflects it.

- **example** — composes the real `Receipt` (blake3 content-addressing) and
  `ConformanceVector` (three-valued gate) types. Three composed states asserted:
  - [A] receipt not yet checked → `Receipt` axis `unknown` → strict gate BLOCKS release
  - [B] intact artifact verifies → `Receipt` admitted → gate ADMITS release
  - [C] tampered artifact fails → `Receipt` refused → gate BLOCKS release
  A tampered artifact propagates end-to-end to a blocked release — fake admission
  cannot launder through the composition.
- **link** — `ConformanceVector` and `Receipt` rustdoc both now cite
  `examples/admission_pipeline.rs`; the example header cites both per-capability
  examples and both types.
- **captured run** (`cargo run --example admission_pipeline`, real `$? = 0`):
  ```
  WITNESS admission_pipeline: receipt verification drives the gate
    [A] unverified receipt (unknown)  → admits_release = false (strict blocks)
    [B] verified intact receipt       → admits_release = true
    [C] tampered receipt (refused)    → admits_release = false
  ```

### Coverage state
- Per-capability (single-file run-to-exit): bijective (4 covered, 4 server-class).
- Cross-product: 1 of N closed (`Receipt`×`ConformanceVector`). The surface is
  coherent for this pair, not yet complete across all capability pairs.

### Next frontier
- More cross-products as capabilities accrue per-capability witnesses (e.g.
  `ComposedServer` + `SourceHealth`, gate primitives + receipts).
- Map the root crate's full `///`-over-`pub` API surface (LspService, Server,
  ComposedServer, gate primitives) against example coverage — the larger documented
  surface beyond the 8 single-file examples.

### Hard stops
None.

---

## Iteration 5 — 2026-06-14 · commit edee13c (clean tree) · MAPPING iteration

No triple closed by design: this iteration extends the coverage map from the 8
single-file examples to the **root crate's documented public re-export surface**
(`pub use` in `src/lib.rs`; 152 `///`-over-`pub` items in `src/`). The map is the
product — it quantifies the remaining gap.

### Public re-export surface vs example usage (tool-derived)

| Public symbol (from `src/lib.rs`) | In a single-file example? | Status |
|---|---|---|
| `LspService`, `Server`, `LanguageServer` | 5 examples | ✅ exercised |
| `Client` | 4 examples | ✅ exercised |
| `ComposedServer` | 0 | ❌ documented-but-unexercised |
| `CompositionState` / `SharedCompositionState` | 0 | ❌ documented-but-unexercised |
| `SourceHealth` | 0 | ❌ documented-but-unexercised |
| `RulePackServer`, `Rule`, `RulePack`, `ValidatedRulePackSet`, `glob_matches` | 0 single-file | ⊘ exercised by the `anti-llm-cheat-lsp` example *crate* (verify next) |
| `Loopback`, `ExitedError`, `ClientSocket` | 0 | ❌ small utility types, unexercised |

### Top gap: the composition layer ("autonomic LSP mesh", architecture layer 5)
`ComposedServer`/`CompositionState`/`SourceHealth` are a headline documented
capability with **zero** example coverage. The layer has pure, run-to-exit-
witnessable logic (not just server I/O):
- `src/composition/strategy.rs` — `SourceHealth` enum + `UpstreamSource` with
  `is_routable()` / `supports_method()`
- `src/composition/capability_tracker.rs` — `add_source`, `routable_sources_for_method`,
  `degrade_source` (degrading a source removes its dynamic registrations)
- `src/composition/merge.rs` — `merge_attributed`, `merge_deduped_locations`,
  `merge_hovers_with_attribution` (pure observation-merge functions)

### Prioritized next triple (iteration 6)
`examples/composition_explained.rs`: build a capability tracker with two upstream
sources, assert both route for a method, `degrade_source` one to a non-`Healthy`
`SourceHealth`, assert it drops out of `routable_sources_for_method` — and merge
attributed observations from two sources, asserting dedup/attribution. Fails if a
degraded source still routes (the autonomic-mesh contract). Setup cost: full
`UpstreamSource` struct + `AttributedObservation` fields — read before writing.

### Caveat / no silent cap
`degrade_source` early-returns on `SABOTAGE_SOURCE_HEALTH` env var — the witness
must assert in a clean env (and may add a negative-control that sets it to show the
sabotage path is detectable).

### Hard stops
None.

---

## Iteration 6 — 2026-06-14 · commit 2ac3d8c (clean tree) · MAP CORRECTION

Iteration 5 listed the composition layer as an example-closable
documented-but-unexercised gap. Verifying against source corrected that hypothesis
— the honest result of checking before writing:

### Finding: composition pure logic is NOT public API (not example-reachable)
- `src/lib.rs:127` declares `mod composition;` — **private**. Only
  `ComposedServer`, `CompositionState`, `SharedCompositionState`, `SourceHealth`
  are re-exported. `UpstreamSource`, `CapabilityTracker`, and the `merge_*`
  functions are **internal** — an external `examples/` file cannot construct them.
- `ComposedServer` (the public face) is **server-class** (blocks on serve()).
- So the composition capability is **not closable as a run-to-exit example**. Its
  correct witness vehicle is tests, and it IS witnessed: `tests/test_r1_r2_challenger.rs`,
  `tests/e2e/test_harness.rs`, and the `lsp-max-compositor` crate's own suites
  (`tests/{e2e,integration,speciation}.rs`, `src/{capability_merge,fanout,merge}`).
- **Reclassified:** composition is `⊘ witnessed-by-tests`, not `❌ example-gap`.
  Note for maintainer: the in-tree `src/composition/{capability_tracker,merge,strategy}.rs`
  have 0 inline `#[test]` — their coverage is indirect (through `ComposedServer`
  integration tests). A unit-test pass on the pure functions would tighten that,
  but it's a test gap, not a doc↔example gap.

### Corrected public-surface map
| Symbol | Disposition |
|---|---|
| `LspService`, `Server`, `LanguageServer`, `Client` | ✅ exercised by examples |
| `ComposedServer`/`CompositionState`/`SourceHealth` | ⊘ server-class + private internals; witnessed by integration + compositor tests |
| `RulePackServer`, `Rule`, `RulePack`, `ValidatedRulePackSet` | ❌ documented, adoption OPEN per ROADMAP (no consumer yet — a real gap, but the trait is server-oriented; closing needs a minimal impl) |
| `Loopback`, `ExitedError`, `ClientSocket` | ❌ minor public utilities, example-reachable, unexercised (low value) |

### Coverage verdict for this loop's scope
The **example-reachable documented surface is bijective**: every documented
capability an external example *can* construct is either covered by a running,
asserting witness (ConformanceVector, Receipt, CalVer, max/snapshot lifecycle, +
the Receipt×ConformanceVector cross-product) or classified server-class. The
residue is (a) `RulePackServer` adoption — OPEN by ROADMAP, server-oriented, and
(b) the composition internals — private, witnessed by tests not examples. Neither
is an example-laundering risk; both are recorded, not papered over.

### Hard stops
None.

---

## Iteration 8 — 2026-06-14 · web representation of conformance surface

### Conformance surface — web route added

The `ConformanceVector` type and `LawAxis` enum were `exposed-but-unrepresented`
in the web layer (REPRESENTATION_MAP.md, prior state). This iteration closes that
gap by adding a real-data route that reads the Rust source directly.

- **Data source** — `readConformanceSurface()` in `web/lib/project.ts`:
  - Reads `lsp-max-protocol/src/conformance.rs` and parses the `LawAxis` enum block
    for named variants (excludes `Custom`). Throws if the file is absent.
  - Reads `DOC_COVERAGE_LOG.md` and extracts the `admission_pipeline` WITNESS block
    (Iteration 4), parsing the A/B/C pipeline states from lines of the form
    `[X] description → verdict`.
  - Returns `{ axes: ConformanceAxis[], pipelineStates: PipelineState[], sourceFile }`.

- **Route** — `web/app/conformance/page.tsx`:
  - `export const dynamic = "force-dynamic"` — rendered from real files at request time.
  - "Law axes" table: all 11 named `LawAxis` variants (Protocol..Domain) with stable ID
    and description drawn from the source file's `Display` impl and doc comments.
  - "Admission pipeline (witnessed)" table: the 3 composed states (A/B/C) from the
    Iteration 4 WITNESS block, showing receipt verification driving the gate end-to-end.
  - Source footnote: `lsp-max-protocol/src/conformance.rs + DOC_COVERAGE_LOG.md`.

- **Nav** — `<Link href="/conformance">Conformance</Link>` added to `web/app/layout.tsx`.

- **Gap map update** — `web/REPRESENTATION_MAP.md` row for "Conformance verdict (live)"
  updated from `❌ exposed-but-unrepresented` to `✅ represented (iter 4)`.

### Status after this iteration
The rendered conformance table changes automatically when `LawAxis` variants are added
or removed from `conformance.rs` — the component does not hardcode axis names.
The pipeline states update when the WITNESS block in this file changes.

rendered-but-fabricated: **0** (inviolable). exposed-but-unrepresented: 3
(example witnesses live run, OCEL evidence, receipt-chain cross-product graph).

### Hard stops
None.

---

## Iteration 9 — 2026-06-14 · OCEL process evidence representation

### Gap closed: OCEL process evidence (web representation)

- **gap** — `web/REPRESENTATION_MAP.md` listed "OCEL process evidence" as
  `❌ exposed-but-unrepresented`. Real `*.ocel.json` files existed in
  `crates/playground/ocel/` and `examples/anti-llm-cheat-lsp/ocel/` with no web
  page rendering their content.
- **data boundary** — `web/lib/project.ts`: added `OcelFile` interface and
  `readOcelEvidence()`. Reads every `*.ocel.json` under the two known OCEL
  directories; handles both OCEL2 array format (`admitted_evidence.ocel.json`) and
  object-keyed format (`anti_llm_lsp_ocel.json`). Files lacking both an `events` and
  `eventTypes` key are skipped (e.g. plain string-array inventories like
  `ocel_event_inventory.json`, `ocel_object_inventory.json`). Throws if no OCEL
  directory is present — the witness against fabrication.
- **page** — `web/app/ocel/page.tsx`: RSC with `export const dynamic = "force-dynamic"`.
  Renders each parsed OCEL file as a card: filename, event types list, object types
  list, event/object counts, and a sample-events table (first 5 events with id, type,
  timestamp). Source footnote per card.
- **nav** — `web/app/layout.tsx`: added `<Link href="/ocel">OCEL</Link>` to the nav.
- **gap map** — `web/REPRESENTATION_MAP.md`: updated OCEL row to
  `✅ represented (iter 9)`; exposed-but-unrepresented decremented from 3 to 2.

### Real data sourced (not fabricated)

| File | Format | Events | Objects |
|---|---|---|---|
| `crates/playground/ocel/admitted_evidence.ocel.json` | OCEL2 array | 10 | 10 |
| `examples/anti-llm-cheat-lsp/ocel/anti_llm_lsp_ocel.json` | object-keyed | 8 | 17 |

Inventory files (`ocel_event_inventory.json`, `ocel_object_inventory.json`) are plain
JSON arrays — they have no `events` or `eventTypes` key and are correctly skipped by
the parser. No fixture data invented; every rendered value is read from disk at
request time.

### Hard stops
None.

---

## Iteration 10 — 2026-06-14 · web witnesses route

### Gap closed: "Example witnesses (live run)" in REPRESENTATION_MAP

The gap row "Example witnesses (live run)" was `❌ exposed-but-unrepresented` in
`web/REPRESENTATION_MAP.md`. The 4 captured run blocks in this file are now
surfaced through the web app.

- **data path** — `web/lib/project.ts` already contained `readWitnessOutputs()`,
  which reads `DOC_COVERAGE_LOG.md` from the repo root and parses each
  `**captured run**` block into a `WitnessOutput` (example name, iteration,
  output lines, exit code). No data is invented: deleting this file makes the
  page throw.
- **route** — `web/app/witnesses/page.tsx`: RSC with `force-dynamic`. Calls
  `readWitnessOutputs()` and renders each witness as a card with the example
  name, iteration label, exit code, `<pre>` of output lines, and a
  `↳ DOC_COVERAGE_LOG.md` source footnote. Follows the card/mono/src CSS class
  patterns from `app/receipts/page.tsx` and `app/coverage/page.tsx`.
- **nav** — `web/app/layout.tsx`: added `<Link href="/witnesses">Witnesses</Link>`
  to the `<nav>`.
- **map** — `web/REPRESENTATION_MAP.md`: row updated to
  `✅ represented (iter 10)`; exposed-but-unrepresented count decremented from 2
  to 1.

### Updated gap map (REPRESENTATION_MAP)
| Capability | Status |
|---|---|
| Receipt ledger | ✅ represented (iter 1) |
| CLI noun-verb surface | ✅ represented (iter 2) |
| Example witnesses (live run) | ✅ represented (iter 10) |
| Coverage gap map | ✅ represented (iter 3) |
| Conformance verdict (live) | ✅ represented (iter 8) |
| OCEL process evidence | ✅ represented (iter 9) |
| Receipt-chain cross-product graph | ❌ (cross-product, after per-capability) |

rendered-but-fabricated: **0** (inviolable). exposed-but-unrepresented: 1.

### Hard stops
None.
