# wasm4pm Cognitive-Breed Configuration — Plan

**Status: BLOCKED** — the breed registry and the entire cognition surface live in the
sibling repo `../wasm4pm`, which is **absent** in this workspace. No breed artifact is
written into `lsp-max`; this file is the reverse-engineered specification + generation
plan, derived from the in-repo executable law `src/diagnostics/cognition_laws.rs`
(its parser is treated as ground truth). All live results are UNKNOWN until
`../wasm4pm` (and `../wasm4pm-compat`, for `FRESH_NAME_PAIRS`) are checked out.

## Registry schema (`crates/wasm4pm-cognition/breeds/registry.json`)

Three accepted shapes: top-level array `[ {…} ]`; wrapper `{ "breeds": [ … ] }`;
object-map `{ "<breed_id>": { … } }` (the map key is injected as `breed_id`).

- Id field: `breed_id` **or** fallback `id`. An entry with neither is silently skipped.
- `status`: defaults to `"UNKNOWN"`. **Only `"PARTIAL_ALIVE"` triggers the COG checks**
  (strict equality, not a threshold); every other status is skipped.

```jsonc
{ "breeds": [
  { "breed_id": "ltl_monitor", "status": "PARTIAL_ALIVE", "module": "ltl_monitor", "lens": "LTL" }
]}
```

## Dispatch arm shape (`crates/wasm4pm-cognition/src/breeds/dispatch.rs`)

The parser maps `module::Struct` use-decls to a struct→module table, then binds
arms `"<bid>" => run_breed(&Struct, …)` **or** `"<bid>" => Struct.run(…)`. The
resolved module stem drives the COG-001 module path `src/breeds/<stem>.rs`. COG-012
presence is satisfied by the literal `"<bid>"` **or** `BreedId::<PascalCase(bid)>`.

```rust
use crate::breeds::{ ltl_monitor::LtlMonitor };
match breed { "ltl_monitor" => run_breed(&LtlMonitor, input), }
```

## Consistency constraints (per PARTIAL_ALIVE breed)

1. `[ASP]` `PARTIAL_ALIVE(b) → dispatch_arm(b)` (COG-012) — **OPEN**
2. `[ASP]` `PARTIAL_ALIVE(b) → exists(src/breeds/<stem_from_dispatch(b)>.rs)` (COG-001) — **OPEN**
3. `[FRAMES]` id-field present and `status` literal-correct (parser tolerates absence by skipping) — **PARTIAL**
4. `[ASP]` id uniqueness across shapes (object-map silently collapses duplicate keys) — **UNKNOWN**
5. `[META]` `FRESH_NAME_PAIRS[bid]` strings MUST NOT appear non-comment in `<stem>.rs` (COG-010 / A8) — **OPEN**
6. `[ASP]` DoD: module ∧ ocpn ∧ report ∧ `admitted=true` ∧ fixture (COG-011) — **OPEN**

## DoD artifact requirements (laws COG-002 … COG-011)

| Artifact | Path (wasm4pm root) | Law | Required fields |
|---|---|---|---|
| OCPN model | `ocel/models/l1/<breed>.ocpn.json` | COG-002 | existence |
| Fitness report | `ocel/reports/<breed>.json` | COG-003/006/007/011 | `fitness == 1.0`; `measured_by` + `measured_on` + (`run_id` or `provenance.run_id`); `admitted == true` for DoD |
| Rust paper fixture | `crates/wasm4pm-cognition/tests/fixtures/papers/<breed>.json` | COG-004/005 | one of `expected`/`expected_value`/`asserted_value`/`paper_value` (A12) |
| TS fixture mirror | `packages/cognition/src/__tests__/fixtures/papers/<breed>.json` | COG-009 | existence (mirror of Rust fixture) |
| Docs card | `docs/breeds/<breed>.md` | COG-008 | existence |
| Module purity | `crates/wasm4pm-cognition/src/breeds/<module>.rs` | COG-010 | no non-comment `FRESH_NAME_PAIRS` name for the breed (A8) |

### Templates (measurement-gated values stay `UNKNOWN_UNTIL_MEASURED` — never fabricate a passing fitness)

```json
// ocel/reports/<breed>.json
{
  "breed_id": "<breed>",
  "fitness": "UNKNOWN_UNTIL_MEASURED — emit 1.0 only from a real OCEL conformance run",
  "admitted": "UNKNOWN_UNTIL_MEASURED — true only after fitness==1.0 is verified",
  "measured_by": "<conformance-engine-id@version>",
  "measured_on": "UNKNOWN_UNTIL_MEASURED — ISO-8601 UTC of the run",
  "provenance": { "run_id": "UNKNOWN_UNTIL_MEASURED", "model": "ocel/models/l1/<breed>.ocpn.json", "log": "<ocel log path>" }
}
```
```json
// tests/fixtures/papers/<breed>.json   (A12: a citation must assert a value)
{
  "breed_id": "<breed>",
  "paper": { "citation": "<author, title, year, DOI>" },
  "expected": { "value": "UNKNOWN_UNTIL_MEASURED — the value the paper asserts", "unit": "<unit>", "source_locus": "<page/figure/equation>" }
}
```

## Generation plan (lives in `../wasm4pm`, not here)

1. Read `registry.json`; for each `status == PARTIAL_ALIVE` breed, resolve `<module>`
   via the same dispatch parse logic in `cognition_laws.rs` (source of truth, not naming convention).
2. Scaffold the artifacts from the templates, leaving measurement-gated fields as the
   literal `UNKNOWN_UNTIL_MEASURED` sentinel — the generator never writes a passing `fitness`/`admitted`.
3. Run the real OCEL conformance engine to produce `fitness`, `run_id`, `measured_on`;
   only that step may set `fitness: 1.0` / `admitted: true`, and it must emit a receipt
   (digest + boundary, cf. `examples/wasm4pm-lsp/tests/receipts.json`) as COG-007/COG-011 evidence.
4. Load `FRESH_NAME_PAIRS` from `wasm4pm_compat::fresh_names`; assert no breed's
   fresh-name appears non-commented in its module before scaffolding (pre-flight COG-010 / A8).
5. Re-run `audit_breeds()`; report counts via `AuditSummary`, never as "all clear".

## Release-gate wiring (Area 10)

`AuditSummary::is_release_gate_green()` is `error_count == 0 && a8_violations == 0`
and currently has **no caller**. Two gaps, recorded for a future (build-verified) change:

- `[META]` **Emptiness collapse (BLOCKED, live):** `audit_breeds` returns an empty
  vec when `registry.json` is absent/unreadable or `dispatch.rs` is unreadable, and an
  empty audit yields `is_release_gate_green() == true`. With `../wasm4pm` absent,
  emptiness masquerades as conformance — an `UNKNOWN → ADMITTED` collapse. The gate
  must report **UNKNOWN** (not green) when no registry is found.
- `[ASP]` The predicate ignores `a10_violations`, `a12_violations`, and the
  `total_partial_alive == 0` ("nothing audited") case.

Proposed three-valued predicate (apply only with a build to verify — not applied here):

```text
ADMITTED ⟺ registry_present ∧ parsable ∧ total_partial_alive > 0
           ∧ error_count = 0 ∧ a8 = 0 ∧ a10 = 0 ∧ a12 = 0
otherwise ∈ { UNKNOWN, REFUSED }   // never green
```

ANDON wiring (the config half is applied in `lsp-max.toml`): the `COG-` prefix is now
declared on the `wasm4pm-lsp` entry, but it is **inert** until that server actually
emits `COG-*` codes (it does not call `audit_breeds` today) — the emission half is
BLOCKED on `../wasm4pm`.
