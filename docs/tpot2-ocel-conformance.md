# TPOT2 OCEL Conformance Grounding (Object-Centric Process Mining)

Status: CANDIDATE. Core (`src/pipeline/ocel.rs`) is ADMITTED-in-isolation by
`scripts/tpot2-harness-verify.sh` (10 unit tests, including negative controls).
Engine-backed alignment conformance is UNKNOWN without the wasm4pm engine.

## Why this exists

The scalar (`search.rs`) and Pareto (`pareto.rs`) searches score breed pipelines
by **composition** alone. The heuristic fitness path previously **ignored the
OCEL log entirely** — "fitness for OCEL event logs" only held when the wasm4pm
engine was present. `ocel.rs` closes that gap: when an OCEL log is available it
grounds fitness in the **log's own object-centric structure**, in the spirit of
van der Aalst's object-centric process mining (OCPM / OCEL 2.0).

## What it reads

A minimal OCEL 2.0 reader (`OcelLog`: `events` + `objects`, with `type`, `time`,
and `relationships → objectId`). It tolerates missing fields via serde defaults
and matches `tests/fixtures/tpot2/sample.ocel.json`.

Negative control: `read_ocel_log` returns `None` for an absent or unparseable
source — an absent observation source is never fabricated into a log, and the
caller stays on a lower-grounding evaluator.

## The structural profile (`LogProfile`)

Each object's events are ordered by timestamp into a trace; the **object-centric
directly-follows graph (OC-DFG)** is read off those traces. Every signal is
bounded to `[0,1]`:

| Signal | OCPM notion | Definition |
|---|---|---|
| `activity_variety`  | activity set        | distinct activities / cap |
| `object_type_spread`| object types        | distinct object types / cap |
| `temporal_density`  | lifecycle length    | fraction of object traces with ≥2 events |
| `divergence`        | OCPM **divergence** | fraction of traces where an activity repeats |
| `convergence`       | OCPM **convergence**| fraction of events shared by ≥2 same-type objects |
| `df_density`        | footprint density   | distinct OC-DFG pairs / activities² |

`convergence` and `divergence` are van der Aalst's object-centric terms:
convergence is one event over many like objects; divergence is one object seeing
the same activity more than once.

## Demand-match fitness

`LogProfile::demand_match(breeds)` scores how well a pipeline's cognitive-category
coverage (via the authoritative `catalog::breed_category`) meets the **demands of
this specific log**:

- a temporally/divergently structured log rewards a **Temporal** breed;
- an object-centrically complex log (spread / convergence / dense DFG) rewards
  **broad category coverage**.

Tested monotonic properties: on a temporally-demanding log, adding a Temporal
breed strictly raises fitness; broader coverage does not lower it; an empty
pipeline scores 0.0; all outputs are bounded.

## Wiring

`fitness::auto_evaluator(ocel_path)` now selects, in descending grounding order:

1. wasm4pm-cli present → `SubprocessFitnessEvaluator` (engine-backed).
2. else OCEL present + carrying structure → `LogGroundedFitnessEvaluator`.
3. else → `HeuristicFitnessEvaluator` (log-blind composition heuristic).

## Honesty boundary

Every signal here is a **structural proxy** computed from the log's shape. It is
**not** engine-backed alignment conformance (replay fitness / precision), which
requires wasm4pm. The proxy never presents itself as an admitted verdict; the
status mapping keeps an unverifiable outcome `UNKNOWN`.

| Element | Status |
|---|---|
| OCEL 2.0 parse + negative control | ADMITTED (harness) |
| OC-DFG + convergence/divergence signals (bounded) | ADMITTED (harness) |
| demand-match monotonicity | ADMITTED (harness) |
| `auto_evaluator` log-grounded selection | ADMITTED (harness) |
| Engine-backed alignment fitness/precision | UNKNOWN (needs wasm4pm) |
