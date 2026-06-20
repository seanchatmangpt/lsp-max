# REPRESENTATION_MAP

Faithful Next.js representation of the **lsp-max** project. The rule: every
rendered claim is witnessed by real project data — its data path terminates
*outside the component* in an actual artifact, binary, or type. A component that
would render identically if the project were deleted is fabrication and is not
allowed. Each entry cites the real source its data comes from.

App lives in `web/` (App Router, RSC). Data paths resolve to the repo root
(`web/..`) so server components read the project's genuine artifacts.

## Exposed real surface (enumerated by tool)

| Real source | Path / command | Renderable as |
|---|---|---|
| Receipt artifacts (8) | `receipts/*.receipt.json`, `crates/playground/receipts/GALL-CHECKPOINT-*.receipt.json` | receipt ledger: digest, status, claims, replay_pointer |
| OCEL evidence | `crates/playground/ocel/*.ocel.json`, `examples/anti-llm-cheat-lsp/ocel/*` | process-evidence view |
| CLI noun-verb surface (19 nouns) | `crates/lsp-max-cli/src/nouns/*.rs` | command-surface map |
| Example witnesses (run-to-exit) | `cargo run --example {conformance_vector,receipt_chain,calver_law,admission_pipeline}_explained` | live witness output (server action) |
| Doc↔example coverage | `DOC_COVERAGE_LOG.md` | coverage gap map |
| Protocol types | `lsp-max-protocol/src/{conformance,core}.rs` | type surface (ConformanceVector, Receipt, LawAxis) |
| Workspace version | `Cargo.toml` workspace.package.version (CalVer 26.6.9) | version banner |

## Gap map (rendered ↔ exposed)

Metric: drive both directions to zero. `❌ exposed-but-unrepresented` = real
capability with no UI. `⚠ rendered-but-fabricated` = UI with no real source (must
be zero at all times — this is the inviolable rule).

| Capability | UI component | Status |
|---|---|---|
| Receipt ledger | `app/receipts` (RSC reads real `*.receipt.json`) | ✅ represented (iter 1) |
| CLI noun-verb surface | `app/cli` (RSC parses real `nouns/*.rs`) | ✅ represented (iter 2) |
| Example witnesses (live run) | `app/witness` (server action runs real `cargo run --example`) | ✅ represented (iter 4) |
| Coverage gap map | `app/coverage` (RSC parses real DOC_COVERAGE_LOG.md) | ✅ represented (iter 3) |
| Conformance verdict (live) | — | ❌ exposed-but-unrepresented |
| OCEL process evidence | — | ❌ exposed-but-unrepresented |
| Receipt-chain cross-product graph | — | ❌ (cross-product, after per-capability) |

rendered-but-fabricated: **0** (inviolable). exposed-but-unrepresented: 3.

## Iteration log

### Iteration 1 — scaffold + receipt ledger
- Scaffolded Next.js 16 App Router (RSC) in `web/`.
- `web/lib/project.ts`: typed boundary reading real receipt JSON from the repo
  root; throws if the directory/shape is absent (witness: deleting `receipts/`
  breaks the page).
- `app/receipts/page.tsx`: server component renders the 8 real receipts.
- Witness: data path is `fs.readFile(<repo>/receipts/*.receipt.json)` — no fixtures.

### Iteration 1 — render witness + finding
- Build: `npm run build` ✓; `/` and `/receipts` are `ƒ Dynamic` (server-rendered
  from real files at request time, `force-dynamic`).
- Render witness (server started, HTML inspected): `/receipts` rendered real
  identifiers `COMPOSITOR-SCALE-ADMITTED-26.6.9`, `GALL-CHECKPOINT-003..008`,
  `perf-refactors`, status `ADMITTED`, and the real benchmark claim text `CS1
  deposit_contention …` — all parsed from actual `*.receipt.json`. `/` rendered
  7 real receipts, 2 ADMITTED, version 26.6.9 from `Cargo.toml`.
- **Finding (real source is local-only):** root `.gitignore` line 13 `**/receipts/`
  gitignores the receipt artifacts — `git ls-files receipts/` returns 0. The UI
  renders real on-disk artifacts, but they are NOT in version control, so a fresh
  clone would render an empty ledger. Not fabrication (files are real), but the
  witness is environment-local. Recorded, not faked. (The route source
  `web/app/receipts/page.tsx` was force-added past the same ignore rule.)

### Iteration 2 — CLI surface view
- `readCliSurface()` parses `#[verb("…")]` over `pub fn` from the real
  `crates/lsp-max-cli/src/nouns/*.rs`; throws if the noun dir is gone.
- `app/cli/page.tsx`: RSC rendering 18 nouns / ~80 verbs with real arg names.
- Render witness (HTML): real nouns (conformance, diagnostics, snapshot,
  telemetry, admission, metamodel), real verbs (breakdown, score, vector), real
  arg `instance_id` ×102 — parsed from source, not invented.
- exposed-but-unrepresented now 5: example witnesses, coverage map, conformance
  (live), OCEL, receipt-chain graph.

### Iteration 3 — coverage view
- `readCoverage()` parses iteration headers + status rows from the real
  DOC_COVERAGE_LOG.md; throws if absent.
- `app/coverage/page.tsx`: RSC rendering iterations + per-item covered/gap status.
- Render witness (HTML): real example items (conformance_vector_explained.rs,
  receipt_chain_explained.rs), real Iteration 1–6 headers, covered/gap counts.
- exposed-but-unrepresented now 4: example witnesses (live run), conformance
  (live), OCEL evidence, receipt-chain cross-product graph.

### Iteration 4 — live witness runner (server action)
- `app/witness/actions.ts` (`"use server"`): `runExample()` spawns
  `cargo run --quiet --example <name>` in the real repo (whitelisted to the 4
  witness examples), returns real stdout + real exit code, including real failure.
- `app/witness/runner.tsx` (`"use client"`, React 19 `useActionState`): buttons
  invoke the action; pending state waits on actual cargo work; renders real output.
- `app/witness/page.tsx`: gates the button list on actual `examples/*.rs` files;
  throws if none present.
- Witness: the action's command (`cargo run --example admission_pipeline`) emits
  real WITNESS output, exit 0; the page (HTTP 200) lists the 4 real examples.
  This is the project *executing*, not a fixture — the frontier feature.
- exposed-but-unrepresented now 3: conformance (live verdict), OCEL evidence,
  receipt-chain cross-product graph.
