# TPOT2 Breed-Pipeline Optimizer — Reference

**Status: PARTIAL.** The library (`lsp_max::pipeline`), the `clap-noun-verb` CLI
noun, the genetic search engine, the fitness functions, the receipt emitter, and
the `just` recipes are present in-tree. Engine-backed (wasm4pm-cli) fitness is
CANDIDATE — it exists in the library but is not yet wired through the CLI verbs
on this branch. No `SUPPORTED_WITH_TRANSCRIPT` claim is made here; admission
claims require a receipt artifact (see [Receipts](#7-receipts)). Per-row bounded
statuses are in [Feature status](#feature-status-of-the-optimizer-itself).

TPOT2 here means a **T**ree-based **P**ipeline **O**ptimization **T**ool 2 style
optimizer: it searches combinations of wasm4pm cognitive *breeds* via genetic
programming, scoring each candidate against a fitness function. The name is
borrowed for the search *shape* (population → fitness → selection → variation);
the breeds and the optional engine are wasm4pm's.

---

## 1. What it is

The optimizer assembles wasm4pm cognitive breeds into ordered *pipelines* and
evolves a population of candidates to find a high-fitness pipeline for an OCEL
event log. It is exposed two ways:

- as a Rust library module, `lsp_max::pipeline` (root crate `src/pipeline/`);
- as a `clap-noun-verb` CLI noun, `lsp-max-cli pipeline` (filename = noun,
  `#[verb]` = action), driven by agents, CI, and release gates.

The editor or agent is a **client** of this surface, not an owner of it. The LSP
surface remains read-only by law (AGENTS.md §4): it may emit diagnostics,
hovers, and code-action *intents*, but it never mutates files. The CLI noun
reports search results and bounded statuses; it does not write into the
workspace on its own. Any future mutation must route through the
`CodeAction → clap-noun-verb admission → … → MutationGate → Receipt` chain.

Pipelines are **linear chains** of breed nodes in the current implementation;
tree-shaped pipelines are CANDIDATE for a later iteration (see
`BreedPipeline` in `src/pipeline/types.rs`).

---

## 2. Architecture

The library lives in `src/pipeline/` and is re-exported as `lsp_max::pipeline`
(`src/lib.rs`). Four modules:

| Module | File | Role |
|---|---|---|
| `catalog` | `src/pipeline/catalog.rs` | The static breed catalog (`KNOWN_BREEDS`) and `BreedCategory` + `breed_category()` partitioning. |
| `types` | `src/pipeline/types.rs` | `BreedPipeline`, `BreedNodeConfig`, `PipelineSearchConfig`, `PipelineSearchResult`, `PipelineEvalResult`, and the `PipelineBoundedStatus` enum. |
| `search` | `src/pipeline/search.rs` | The genetic engine: `PipelineSearch`, the `FitnessEvaluator` trait, the xorshift64 `Prng`, and `DiversityFitnessEvaluator`. |
| `fitness` | `src/pipeline/fitness.rs` | OCEL-oriented evaluators: `BreedFitnessEvaluator` trait, `SubprocessFitnessEvaluator` (wasm4pm-cli), `HeuristicFitnessEvaluator`, and `auto_evaluator()`. |

The CLI noun is `crates/lsp-max-cli/src/nouns/pipeline.rs`. It consumes the
`lsp_max::pipeline` library directly rather than duplicating it:
`list-breeds`/`schema` read `catalog::{KNOWN_BREEDS, breed_category}`, `evaluate`
calls `fitness::auto_evaluator(ocel_path).evaluate(..)`, and `search` drives
`search::PipelineSearch::run` over `KNOWN_BREEDS`. Because `PipelineSearch` takes
the `search::FitnessEvaluator` trait while `auto_evaluator()` returns the
`fitness::BreedFitnessEvaluator` trait (same signature, distinct traits), a
one-method newtype `CliFitnessAdapter` bridges the two so the search engine runs
on the auto-selected (subprocess-or-heuristic) evaluator with no duplicated
scoring logic.

### Search flow

```text
catalog (breed pool)
    │
    ▼
population init        random pipelines, length in [min, max] (CLI: 2..=4)
    │
    ▼
fitness eval           score every candidate in [0.0, 1.0]
    │
    ▼
selection              tournament selection (k draws, best wins)
    │
    ▼
crossover              single-point, parents joined at random splits
    │
    ▼
mutation               point mutation: one node replaced from the pool
    │
    ▼
elitism                top 2 carried unchanged into the next generation
    │
    ▼  (repeat per generation; early-stop when best ≥ admission_threshold)
    │
    ▼
bounded-status result  ADMITTED | PARTIAL | REFUSED | UNKNOWN
```

The `PipelineSearch` engine (`search.rs`) is deterministic for a given seed:
identical seeds reproduce identical runs (`Prng` is xorshift64 seeded with a
fixed mix constant). It re-evaluates only pipelines whose fitness sentinel was
reset by mutation/crossover, and early-stops as soon as `best_fitness` reaches
`admission_threshold`.

---

## 3. The breed catalog (57 breeds, 7 categories)

`KNOWN_BREEDS` (`src/pipeline/catalog.rs` and the mirror in the CLI noun) lists
**57** breed string IDs that match wasm4pm-cognition's dispatch IDs. Breeds are
partitioned into **7** categories. `breed_category()` in the library maps known
names to a `BreedCategory`; unknown names default to `MetaBased`. The CLI noun
calls this same `breed_category()`, so the CLI and library agree on every breed's
category by construction.

| `BreedCategory` | Example breeds |
|---|---|
| `LogicBased` | `asp`, `prolog`, `description_logic`, `sat_cdcl`, `tableaux`, `abductive_ibe`, `abductive_lp`, `clp`, `circumscription`, `default_logic` |
| `RuleBased` | `production_rules`, `cbr`, `dendral`, `analogy_sme`, `version_space`, `ebl`, `ilp`, `markov_logic`, `problog` |
| `PlanningBased` | `strips`, `htn_planning`, `gps`, `partial_order_plan`, `contingent_plan`, `situation_calculus`, `event_calculus`, `mdp`, `pomdp`, `rl_symbolic` |
| `Probabilistic` | `bayesian_network`, `dempster_shafer`, `fuzzy_logic`, `qualitative_reason` |
| `Temporal` | `ltl_monitor`, `ctl_check`, `allen_temporal`, `naive_physics` |
| `MemoryBased` | `frame`, `frames_inheritance`, `hearsay`, `soar`, `act_r`, `episodic_memory`, `script_sam`, `construction_grammar`, `morphological` |
| `MetaBased` | `meta_reasoning`, `belief_merging`, `triz`, `csp_ac3` (and any unrecognized breed) |

Catalog breeds not enumerated above (for example `autoinstinct_*`,
`oracle_chain`, `standing`, `ocpm_route_discoverer`) fall to `MetaBased` via the
default arm. The canonical, full list is the `KNOWN_BREEDS` slice in
`src/pipeline/catalog.rs`; run `lsp-max-cli pipeline list-breeds` for the live
catalog with each breed's category.

---

## 4. Fitness function

The library `HeuristicFitnessEvaluator` (`src/pipeline/fitness.rs`) computes the
bounded score in **[0.0, 1.0]**; the CLI noun delegates to it (via
`auto_evaluator`'s heuristic fallback), so there is one scoring implementation:

```text
fitness = min(1.0,
              diversity     * 0.5      // distinct known categories / 7, capped at 1.0
            + length_score  * 0.4      // 2..=4 nodes optimal
            + temporal_bonus)          // +0.1 if any Temporal breed is present

length_score = 0.0          if len == 0
             = 0.3          if len == 1
             = 1.0          if len in 2..=4
             = min(1.0, 4.0/len)   if len > 4
```

`diversity` counts *distinct* categories among the breeds (the `"unknown"`
category does not contribute), divided by the 7 named categories. Empty
pipelines score `0.0`. The score is clamped so the temporal bonus cannot push a
pipeline above `1.0`.

The library's `search.rs` also ships a separate `DiversityFitnessEvaluator`
(weights `diversity * 0.7 + length_score * 0.3`, optimum at 2–4 *unique* breeds)
used by the engine's own unit tests; it is distinct from the
process-mining-oriented `HeuristicFitnessEvaluator` above.

### Subprocess vs heuristic paths

`src/pipeline/fitness.rs` defines two evaluation paths behind the
`BreedFitnessEvaluator` trait, selected by `auto_evaluator()`:

| Path | When | What it does | Resulting polarity |
|---|---|---|---|
| `SubprocessFitnessEvaluator` | wasm4pm-cli is locatable (`wasm4pm` on PATH, or `../wasm4pm/target/{debug,release}/wasm4pm-cli`) | Runs `wasm4pm breed run <breed> --score-only [--ocel <path>]` per breed, parses `{"fitness": …}` from stdout, averages | Engine-backed score — eligible for an ADMITTED claim **only with a receipt** |
| `HeuristicFitnessEvaluator` | wasm4pm-cli absent, or every breed run failed / produced no score | The composition formula above (no subprocess, no OCEL read) | Composition-only score; not engine evidence by itself |

`SubprocessFitnessEvaluator::evaluate` falls back to the heuristic when the CLI
is absent *or* when no breed produced a parseable score — so a single number from
this evaluator does not, on its own, prove the engine ran. Engine participation
must be witnessed by a receipt, not inferred from the score.

**Honest limitation (CANDIDATE).** The **CLI verbs** `evaluate` and `search` in
`crates/lsp-max-cli/src/nouns/pipeline.rs` route through `auto_evaluator(ocel_path)`,
which spawns `wasm4pm-cli` **only when it answers `--version`** and otherwise
falls back to the heuristic. So when `wasm4pm-cli` is absent (as in CI here),
`ocel_path` is bound into the command/receipt but the score is still heuristic.
Engine-backed fitness over an OCEL log stays CANDIDATE until it is witnessed with
a transcript + receipt from a run where `wasm4pm-cli` was present.

---

## 5. Bounded statuses

The optimizer emits only bounded statuses — never unbounded closure words. The
library type is `PipelineBoundedStatus` (`src/pipeline/types.rs`); the CLI emits
the same words as strings.

| Status | Meaning | Where it is produced |
|---|---|---|
| `ADMITTED` | Best fitness `>= admission_threshold` (library default `0.7`). | `search.run()` when threshold met; CLI `evaluate` when `fitness >= 0.7`. |
| `PARTIAL` | Search ran but stayed below threshold (in-progress / partial convergence). | `search.run()` after all generations below threshold; CLI `evaluate` when `0.3 <= fitness < 0.7`; CLI `search` when `0.0 < best < 0.7`. |
| `UNKNOWN` | Gap in tracing or a precondition not met — for example OCEL log missing, or a non-empty pool whose fitness is too low to read as PARTIAL. | CLI `evaluate` when `0.0 < fitness < 0.3` on a non-empty pipeline; setup check when no OCEL path is configured. |
| `REFUSED` | Hard failure: empty breed pool / empty pipeline / unrecoverable state. | `search.run()` and CLI `search` when the pool is empty; CLI `evaluate` on an empty breed list. |
| `BLOCKED` | An ANDON gate condition blocks evaluation; resolve active `WASM4PM-*` / `GGEN-*` diagnostics before retrying. | Gate / setup surfaces (`PipelineBoundedStatus::Blocked`); the gate is checked before shell actions. |

**`UNKNOWN` never collapses into `ADMITTED` or `REFUSED`.** A missing OCEL log,
an unbuilt engine, or an untraced precondition is reported as `UNKNOWN` — it is
not silently treated as a pass (`ADMITTED`) or as a hard failure (`REFUSED`).
This three-valued discipline mirrors `ConformanceVector`'s
admitted/refused/unknown axes (AGENTS.md): a gap is a third state, not a
polarity.

---

## 6. CLI usage

The noun is `pipeline`; the four verbs are `list-breeds`, `evaluate`, `search`,
and `schema`. Output is JSON (shapes below are illustrative; fitness numbers are
examples, not measured results).

### `list-breeds` — enumerate the catalog

```sh
lsp-max-cli pipeline list-breeds
# just recipe:
just pipeline-breeds
```

```json
{
  "breeds": [
    { "name": "abductive_ibe", "category": "logic" },
    { "name": "cbr", "category": "rule" },
    { "name": "ltl_monitor", "category": "temporal" }
  ],
  "total": 57
}
```

### `evaluate` — score a specific breed sequence

```sh
lsp-max-cli pipeline evaluate --breeds cbr,ltl_monitor,asp
lsp-max-cli pipeline evaluate --breeds cbr,ltl_monitor,asp --ocel-path path/to/log.jsonocel
# just recipe:
just pipeline-evaluate breeds="cbr,ltl_monitor,asp"
```

```json
{
  "pipeline": {
    "id": "pipe-eval-cbr-ltl_monitor-asp",
    "breeds": ["cbr", "ltl_monitor", "asp"],
    "fitness": 0.83
  },
  "status": "ADMITTED"
}
```

(`--ocel-path` is accepted but not read by the CLI evaluator on this branch — see §4.)

### `search` — evolve a population

```sh
lsp-max-cli pipeline search --generations 10 --population-size 20
# just recipes:
just pipeline-search generations="20" pop="30"
just pipeline-quick        # generations 5, population 10
```

```json
{
  "status": "PARTIAL",
  "best_pipeline": {
    "id": "pipe-best",
    "breeds": ["cbr", "ltl_monitor", "asp"],
    "fitness": 0.66
  },
  "best_fitness": 0.66,
  "generations_run": 10,
  "evaluations": 191,
  "summary": "search: PARTIAL after 10 gens, 191 evals"
}
```

### `schema` — defaults and search parameters

```sh
lsp-max-cli pipeline schema
# just recipe:
just pipeline-schema
```

```json
{
  "version": "26.6.9",
  "breed_count": 57,
  "default_generations": 10,
  "default_population_size": 20,
  "admission_threshold": 0.7,
  "fitness_strategy": "heuristic (wasm4pm-cli subprocess when available)"
}
```

### Readiness and receipts

```sh
just pipeline-check                                          # scripts/pipeline-setup.sh — bounded per-prerequisite status
just pipeline-receipt "cbr,ltl_monitor,asp" 0.85 ADMITTED   # scripts/pipeline-receipt.sh — emit a marker receipt
just test-pipeline-receipt                                  # tests/test_pipeline_receipt.sh — receipt emit + validate
```

`pipeline-check` reports each prerequisite with a bounded status (CLI built,
wasm4pm-cli present, OCEL path set, catalog non-empty) and never collapses an
unset OCEL path into ADMITTED.

---

## 7. Receipts

**Test stdout is not a receipt** (AGENTS.md §6). A line such as
`status result: ok`, a logged `ADMITTED`, or a printed fitness number does not
admit a pipeline. The string `ADMITTED` in a search result is a *status word*,
not proof — `StatusWord(ADMITTED) ⇏ Admitted`.

An admission claim needs a **receipt artifact** with bound fields. Emit one with
`scripts/pipeline-receipt.sh`:

```sh
scripts/pipeline-receipt.sh <breeds-csv> <fitness> <status> [ocel_path]
```

It refuses any status that is not bounded
(`ADMITTED|REFUSED|PARTIAL|UNKNOWN|BLOCKED|CANDIDATE|OPEN`) and emits a
marker-style receipt:

```json
{
  "boundary": "-----BEGIN RECEIPT-----",
  "checkpoint": "-----END RECEIPT-----",
  "raw_command": "lsp-max-cli pipeline evaluate --breeds cbr,ltl_monitor,asp",
  "digest": "<64-hex sha256 of breeds|fitness|ocel_path>",
  "digest_algorithm": "sha256",
  "status": "ADMITTED",
  "breeds": "cbr,ltl_monitor,asp",
  "fitness": "0.85",
  "evaluated_at": "<ISO-8601 UTC>",
  "ocel_path": "none"
}
```

Validate the artifact with `scripts/validate-receipt-chain.sh <receipt.json>`,
which checks the boundary/checkpoint markers, a 64-hex digest, the required
bound fields (`digest_algorithm`, `digest`, `raw_command`), and that any status
word is bounded. The validator emits only bounded statuses and reports
**digest freshness as UNKNOWN** — it checks structure, it does not re-run
`raw_command`. A shape it does not recognize (no `boundary`) is reported
`UNKNOWN`, never `REFUSED`, so an unfamiliar shape is not collapsed into a
polarity.

Caveat: `pipeline-receipt.sh` digests `breeds|fitness|ocel_path` — it binds the
*claim*, not a re-executed engine run. A receipt that proves the wasm4pm-cli
engine actually produced the fitness (output digest of a real
`wasm4pm breed run`) is CANDIDATE; the current marker receipt does not assert it.

---

## Feature status of the optimizer itself

Per AGENTS.md §8 / CLAUDE.md, each row carries a bounded status. `tests/test_tpot2_e2e.sh`
produces a transcript + negative controls + a validated marker receipt, but the
receipt artifact is a per-run temp file (not committed in-tree) and binds the
claim rather than a re-executed engine run — so no row claims
`SUPPORTED_WITH_TRANSCRIPT`.

| Row | Piece | Status | Note |
|---|---|---|---|
| TPOT2-LIB | `lsp_max::pipeline` library (`catalog`/`types`/`search`/`fitness`) | ADMITTED | Modules present, re-exported from `src/lib.rs`, covered by in-crate unit tests. |
| TPOT2-CLI | `pipeline` noun + 4 verbs (`list-breeds`/`evaluate`/`search`/`schema`) | ADMITTED | Present in `crates/lsp-max-cli/src/nouns/pipeline.rs`. |
| TPOT2-GA | Genetic engine: tournament + single-point crossover + point mutation + elitism + early-stop | ADMITTED | `PipelineSearch::run` (library); the CLI drives the same engine; deterministic PRNG. |
| TPOT2-FIT-HEUR | Heuristic fitness (diversity ×0.5 + length ×0.4 + temporal 0.1, bounded [0.0,1.0]) | ADMITTED | One formula in the library `HeuristicFitnessEvaluator`; the CLI delegates to it; unit-tested. |
| TPOT2-FIT-ENGINE | wasm4pm-cli subprocess fitness through the CLI verbs | CANDIDATE | CLI `evaluate`/`search` route through `auto_evaluator(ocel_path)`, which uses `SubprocessFitnessEvaluator` only when `wasm4pm-cli` answers `--version` and the heuristic otherwise; engine-backed score stays CANDIDATE until witnessed where the CLI is present. |
| TPOT2-CLI-LIB-UNIFY | CLI noun consuming the `lsp_max::pipeline` library instead of its own copy | ADMITTED | CLI consumes `catalog`/`search`/`fitness`; duplicate catalog/heuristic/GA removed (−215 LOC); bridged by `CliFitnessAdapter`. |
| TPOT2-LSP | Read-only `max/pipeline*` methods + `TPOT2-*` diagnostic family | ADMITTED | `lsp-max-protocol/src/pipeline.rs`: method constants, serde param/result types, `diagnostics_for_search()`; `TPOT2-OCEL-MISSING` holds UNKNOWN distinct from REFUSED/ADMITTED; unit-tested. |
| TPOT2-RECEIPT | Marker-receipt emitter + validator path | PARTIAL | Emitter + validator + integration test present; receipt binds the claim, not a re-executed engine run. |
| TPOT2-RECEIPT-ENGINE | Receipt binding a real `wasm4pm breed run` output digest | CANDIDATE | Not implemented; current digest covers `breeds\|fitness\|ocel_path`. |
| TPOT2-TESTS-RUST | Rust integration test over the library API | ADMITTED | `tests/test_tpot2_pipeline.rs`: 7 cases incl. seed-reproducibility and the empty-pool REFUSED negative control. |
| TPOT2-TESTS-E2E | Shell e2e: CLI verbs + receipt emit/validate + negative controls | PARTIAL | `tests/test_tpot2_e2e.sh` over `tests/fixtures/tpot2/sample.ocel.json`; rejects tampered status, flipped digest, out-of-band emitter status; transcript binds the claim, not an engine run. |
| TPOT2-TREE | Tree-shaped (non-linear) pipelines | CANDIDATE | `BreedPipeline` is a linear chain; tree pipelines noted as a future iteration in `types.rs`. |
| TPOT2-OCEL | OCEL-log-specific fitness end to end | UNKNOWN | Requires `../wasm4pm` engine + an OCEL log; absent here, so end-to-end behavior is untraced (not ADMITTED, not REFUSED). |

Do not read any `ADMITTED` row above as an admission of the *optimizer's
results*. These rows bound the presence and behavior of the code; an admitted
pipeline result for a given OCEL log still requires its own receipt.
