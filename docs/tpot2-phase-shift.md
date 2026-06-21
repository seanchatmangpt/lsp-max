# TPOT2 Phase-Shift Model — Water → Steam (1,700×)

Status: CANDIDATE. Self-contained core (`src/pipeline/phase.rs`) is
ADMITTED-in-isolation by `scripts/tpot2-harness-verify.sh`; the protocol surface
(`lsp-max-protocol/src/phase.rs`, `max/phaseShift`) is BLOCKED-for-build in this
container (sibling `lsp-types-max` absent) but mirrors the verified
`pipeline.rs`/`repair.rs` pattern exactly.

## The metaphor, made precise

Water expands roughly **1,700×** in volume when it crosses the boiling point into
steam. The phase-shift model borrows that picture for the law-state runtime:

- **Boiling point = the admission threshold.** A conformance measurement is a
  "temperature".
- **1,700× expansion = the autonomic-mesh fan-out.** A single *admitted*
  observation propagates as many bounded intents across the layer-5 mesh; an
  observation that has not crossed the boiling point does not expand.

This is a **read-only projection**. It reports a phase and an *intended*
expansion factor; it performs no fan-out and mutates nothing. Actual propagation
routes through the mesh's own hook/action chain.

It is distinct from the runtime's `LspPhase` (the LSP protocol lifecycle,
`Uninitialized → … → Exited`). `ConformancePhase` is the bounded *admission*
state of an observation.

## Phases ↔ bounded statuses (total, three-state preserving)

| ConformancePhase | Matter state | Bounded status | Expansion | Meaning |
|---|---|---|---|---|
| `Frozen`     | solid / ice   | `BLOCKED`  | 0    | ANDON active; no flow until it clears |
| `Liquid`     | water         | `PARTIAL`  | 1    | flowing below the boiling point |
| `Vapor`      | steam         | `ADMITTED` | 1700 | crossed the boiling point; mesh expands it |
| `Unsettled`  | —             | `UNKNOWN`  | 0    | measurement undetermined; never coerced |
| `Decomposed` | —             | `REFUSED`  | 0    | explicit refusal; no longer the same substance |

`Unsettled`/UNKNOWN is its own phase and is **never** folded into `Liquid`/PARTIAL
or `Vapor`/ADMITTED — the three-state law holds.

## Precedence

`phase_for` resolves a world-state with the same precedence the rest of the
admission surface uses (`repair::simulate_admission`):

```
andon_active → Frozen (BLOCKED)
  else refused → Decomposed (REFUSED)
    else unknown → Unsettled (UNKNOWN)
      else conformance ≥ boiling_point → Vapor (ADMITTED)
        else → Liquid (PARTIAL)
```

BLOCKED dominates REFUSED dominates UNKNOWN dominates the boiling-point
comparison. The boiling point is inclusive: `conformance == boiling_point`
admits (`Vapor`).

## Where the conformance signal comes from

The "temperature" is intended to be the breed-pipeline conformance score. When an
OCEL log is present, `src/pipeline/ocel.rs` grounds that score in the log's
object-centric structure (OC-DFG, convergence/divergence) rather than in breed
composition alone (see `docs/tpot2-ocel-conformance.md`). When the wasm4pm engine
is absent, the score is an explicitly-bounded structural **proxy**, and the
status mapping keeps an unverifiable outcome `UNKNOWN`/`Unsettled` rather than
boiling it into `ADMITTED`/`Vapor`.

## Surfaces

- **Library core** — `src/pipeline/phase.rs`: `ConformancePhase`, `PhaseInput`,
  `phase_for`, `phase_shift_report`, `STEAM_EXPANSION_FACTOR = 1700`.
- **Protocol** — `lsp-max-protocol/src/phase.rs`: `max/phaseShift`,
  `PhaseShiftParams`/`PhaseShiftResultMsg`, the `PHASE-*` diagnostic family, and
  `diagnostics_for_phase` (three-state preserving).
- **Mesh (layer 5)** — see `docs/tpot2-autonomic-mesh.md` for how the expansion
  factor models fan-out across the mesh. Dispatch wiring of `max/phaseShift` into
  the runtime method registry is OPEN.

## Status table

| Element | Status |
|---|---|
| `ConformancePhase` total mapping to bounded statuses | ADMITTED (harness) |
| Precedence + boiling-point boundary | ADMITTED (harness) |
| `expansion_factor` (Vapor=1700, Liquid=1, else 0) | ADMITTED (harness) |
| Three-state law (Unsettled never collapses) | ADMITTED (harness) |
| `max/phaseShift` protocol declaration | CANDIDATE (build BLOCKED here) |
| `max/phaseShift` runtime dispatch | OPEN |
| Engine-backed conformance as the temperature | UNKNOWN (needs wasm4pm) |
