# TPOT2 Breed-Catalog Drift Audit

A knowledge-hooks (layer 4) integrity check for the breed catalog.

The in-repo catalog at `src/pipeline/catalog.rs` hard-codes a `KNOWN_BREEDS` slice of
string ids. That slice is a copy of an external truth: the breeds actually defined in the
`wasm4pm-cognition` sibling crate. Copies rot. When upstream adds or removes a breed and
the catalog is not updated, the catalog silently disagrees with reality, and the TPOT2
search space partitioning in `catalog.rs` operates on a stale view.

`scripts/tpot2-catalog-audit.sh` compares the two and reports the disagreement as a
bounded status, so the rot is observable rather than silent.

## What drift means

The audit treats the breed set as two snapshots and reports the symmetric difference:

- **missing from catalog** — a breed id present in the source directory but absent from
  `KNOWN_BREEDS`. Upstream added a breed; the catalog has not caught up.
- **stale / removed upstream** — a breed id present in `KNOWN_BREEDS` but absent from the
  source directory. Upstream removed (or renamed) a breed; the catalog still lists it.

Either direction is drift. Both are reported with the specific ids, not just counts.

## How the source set is derived

The canonical breed set is computed from the source directory:

```
../wasm4pm/crates/wasm4pm-cognition/src/breeds/
```

(The sibling lives at `../wasm4pm` relative to this repo root, per `CLAUDE.md`.)

Each `<name>.rs` file in that directory is breed id `<name>`, with these exclusions —
they are module plumbing and non-breed files, matching how the catalog was originally
derived:

| Excluded entry                     | Why it is not a breed                              |
|------------------------------------|---------------------------------------------------|
| `mod.rs`                           | module wiring                                      |
| `dispatch.rs`                      | dispatch plumbing                                  |
| `registration.rs`                  | registration plumbing                             |
| `registration.rs.backup`          | non-`.rs` backup; the `*.rs` glob already skips it |
| `bayesian_network_test_script.rs`  | a stray test script, not a breed module           |
| `support/` (subdirectory)          | support code; a non-recursive glob skips subdirs   |

The exclusion list mirrors the comment at the top of `src/pipeline/catalog.rs`. If the
exclusion set in the source comment and the script ever diverge, that divergence is itself
a signal to re-derive both.

## How the catalog set is extracted

The in-repo set is read straight from `src/pipeline/catalog.rs`: the quoted string ids
between the `pub static KNOWN_BREEDS` line and its closing `];`. No build step and no Rust
toolchain are required — the audit is dependency-light (`awk`, `grep`, `sed`, `sort`,
`comm`).

## How to run it

From the repo root (where `../wasm4pm` resolves to the sibling checkout):

```sh
bash scripts/tpot2-catalog-audit.sh
```

The script resolves its own location, so it can also be invoked by absolute path from any
working directory. It prints the two source/catalog paths, the two counts, the drift lists
(if any), and a final bounded `CATALOG:` status line. It always exits `0`.

## Bounded outcomes

The audit reports exactly one of three bounded statuses — only the three lines below, with
no unbounded claim.

| Status line                                   | Meaning                                                        |
|-----------------------------------------------|---------------------------------------------------------------|
| `CATALOG: ADMITTED — in sync (N breeds)`     | catalog set and source set are identical; no drift in either direction |
| `CATALOG: PARTIAL — <a> missing, <b> stale`  | drift exists; `<a>` ids missing from catalog, `<b>` ids stale  |
| `CATALOG: UNKNOWN — breed source absent ...` | the source directory could not be observed                     |

`ADMITTED` is bounded to the moment of the run against the observed source. It makes no
claim about future state — re-run after any upstream change.

## Absent source yields UNKNOWN, never silently ADMITTED

If `../wasm4pm/crates/wasm4pm-cognition/src/breeds/` is absent (the sibling is not checked
out), the source set is **unobservable**. The audit reports:

```
CATALOG: UNKNOWN — breed source absent (sibling ../wasm4pm not checked out)
```

This is the load-bearing law of the audit. Absence of the source is a gap in observation —
it is not evidence that the catalog is in sync (`ADMITTED`) and it is not evidence that the
catalog is wrong (`REFUSED`). Collapsing `UNKNOWN` into either polarity would manufacture a
verdict from missing data. The audit refuses to do so and exits `0` with the `UNKNOWN`
line, so a caller running without the sibling sees a gap rather than a false sync.

## Why exit 0 in every case

The audit is a report, not a gate. Returning `0` in all three outcomes makes it composable:
a caller branches on the bounded `CATALOG:` line (parse the status word), not on the exit
code. A drift result is informative signal, not a hard stop, and `UNKNOWN` must never read
as a failure either.

## Future ANDON wiring (CANDIDATE)

The `PARTIAL` line could later feed an ANDON diagnostic family — for example
`TPOT2-CATALOG-DRIFT` — so that catalog drift participates in the gate the way
`WASM4PM-*` and `GGEN-*` diagnostics already do. That wiring is **CANDIDATE**: the audit
emits the bounded signal today, but no diagnostic consumes it and no gate is wired to it
yet. Until that exists, this audit is an out-of-band integrity check, run on demand.
