# TPOT2 Pipeline-Search Benchmark Harness

`scripts/tpot2-bench.sh` is a reproducible harness that sweeps a small parameter
grid over the TPOT2-style breed-pipeline search and records, per cell, what the
`lsp-max-cli pipeline search` verb actually returns. The recorded numbers are
**observations**, not admission claims. The harness asserts nothing about any
specific fitness value or timing, and it binds the run's configuration into a
marker receipt so the configuration is verifiable.

Bounded statuses only. There is no victory claim anywhere in the harness or in
this document.

## What is swept

The grid is the Cartesian product of two axes:

| Axis        | Values     |
|-------------|------------|
| generations | 5, 10, 20  |
| population  | 8, 16, 32  |

That is 9 cells. The grid is kept small so total wall-time stays modest. Each
cell invokes:

```
lsp-max-cli pipeline search --generations <g> --population-size <p>
```

No `--ocel-path` is passed, so the search runs against the library's
auto-selected fitness evaluator. When `wasm4pm-cli` is present on the host the
evaluator uses it as a subprocess; otherwise it falls back to the library
heuristic. The harness does **not** require `wasm4pm-cli` — the heuristic
fallback is an accepted observation source (see the status table below).

## What is measured

For each cell the harness records one row:

- `best_fitness` — the `best_fitness` field the search verb returned (recorded
  verbatim; no claim is made about it).
- `evaluations` — the `evaluations` field (the number of fitness evaluations the
  search performed; this is not always `generations × population`, because the
  search may converge and stop early).
- `status` — the bounded status the search verb returned (`ADMITTED`, `PARTIAL`,
  `UNKNOWN`, `REFUSED`).
- `elapsed_ms` — wall-time for the single `pipeline search` invocation, in
  milliseconds, measured around the subprocess call.

### Timings are environment-dependent observations, not guarantees

`elapsed_ms` reflects the host CPU, system load, whether the binary was built or
located prebuilt, and whether `wasm4pm-cli` is invoked as a subprocess. It is a
point observation on one machine at one moment. It carries no cross-environment
guarantee and must not be read as a performance contract. Re-running on a
different host will produce different numbers; that is expected.

## How to run

From the repository root:

```sh
bash scripts/tpot2-bench.sh
```

The harness will:

1. Build `lsp-max-cli` via `cargo build -p lsp-max-cli`. If the workspace cannot
   build (for example an isolated worktree, or an in-progress tree that does not
   compile), it falls back to a prebuilt `lsp-max-cli` in `target/debug/` or on
   `PATH`. Locating a prebuilt binary is an accepted path; fabricating output is
   not.
2. Capture the binary's `pipeline schema` output (bound into the receipt later).
3. Sweep the 9 grid cells, recording one CSV row per cell.
4. Print the results table.
5. Emit one marker receipt binding the grid and the schema digest, and validate
   it.

Exit codes:

- `0` — at least one cell produced a parseable observation and the receipt
  validated.
- `2` — `BLOCKED`: the harness could not build or locate `lsp-max-cli`.
- non-zero otherwise — `REFUSED`: the schema invocation failed, no cell produced
  a parseable observation, or the receipt did not validate.

The harness is idempotent. All artifacts are written under a `mktemp -d`
directory and removed by an `EXIT` trap, so repeated runs leave no residue.

## How to read the CSV

The results table is written to a temp file as CSV and also printed to stdout.
The column header is:

```
generations,population,best_fitness,evaluations,status,elapsed_ms
```

Column meanings match the "What is measured" section above. A cell whose search
invocation exits non-zero is recorded as a row with `status=BLOCKED` and
`UNKNOWN` placeholders rather than being dropped, so a gap is visible rather than
hidden. A field that could not be parsed from the JSON is written as `UNKNOWN`
for the same reason — `UNKNOWN` is never collapsed into a fitness value or a
polarity.

### Illustrative row (NOT a guarantee, NOT a fabricated measurement)

The following single row is **illustrative only**. It shows the column shape, not
a benchmark result for your host. Do not cite it as a measured number; run the
harness to obtain real observations for your environment.

```
generations,population,best_fitness,evaluations,status,elapsed_ms
ILLUSTRATIVE,ILLUSTRATIVE,<0..1 float>,<int>,ADMITTED|PARTIAL|UNKNOWN,<int ms>
```

The `generations` and `population` cells are written as `ILLUSTRATIVE` on
purpose so this row cannot be mistaken for a real measurement.

## The receipt-binding step

Per project law, the printed table and the stdout lines are **not** a receipt.
The only receipt is the artifact emitted by `scripts/pipeline-receipt.sh` and
checked by `scripts/validate-receipt-chain.sh`.

The harness emits exactly one marker receipt per run. It binds the run's
verifiable configuration into the receipt's content digest:

- the swept grid spec (`generations={5,10,20}xpopulation={8,16,32}`),
- a SHA-256 digest of the captured `pipeline schema` JSON,
- the observed cell count.

These are folded into the receipt's bound `breeds` field, so editing the grid or
changing the binary's schema changes the receipt's content digest. The receipt
carries the bounded status `CANDIDATE`: a benchmark sweep is an observation set,
not an admission of any value. The receipt is then validated; the harness only
exits `0` when `validate-receipt-chain.sh` returns `ADMITTED` for it.

A validated receipt asserts structural binding (boundary marker, checkpoint
closure, 64-hex SHA-256 digest, bound fields, bounded status). It does **not**
assert that any fitness number is good or that any timing is reproducible —
digest freshness is reported `UNKNOWN` by the validator because the source
command is not re-run during validation.

## Bounded status table

| Item                                               | Status    | Note |
|----------------------------------------------------|-----------|------|
| Harness runs the grid and records observations     | ADMITTED  | sweeps 9 cells; writes CSV; binds + validates one receipt; exits 0 |
| Per-cell `best_fitness` / `evaluations` / `status` | ADMITTED  | recorded verbatim from the verb's JSON; no claim made about the values |
| Receipt binding grid + schema digest               | ADMITTED  | one marker receipt per run, validated by `validate-receipt-chain.sh` |
| Cross-environment timing reproducibility           | CANDIDATE | `elapsed_ms` is a single-host point observation; no cross-host guarantee |
| Engine-backed (wasm4pm-cli) timing                 | UNKNOWN   | when `wasm4pm-cli` is absent the heuristic fallback is used; engine timings are not observed |
| Statistical significance of timing deltas          | OPEN      | the harness records single-shot timings, not repeated trials with variance |

Do not collapse `CANDIDATE`, `UNKNOWN`, or `OPEN` into `ADMITTED`. They mark the
boundary of what this harness observes.
