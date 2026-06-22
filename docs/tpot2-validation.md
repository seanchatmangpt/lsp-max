# TPOT2 Validation + Phase-Shift Innovation — Report

Status: bounded. Verification boundary: the full workspace is **BLOCKED** for
build in this container (sibling repos `../lsp-types-max`, `../wasm4pm-compat`,
`../wasm4pm` absent). The self-contained `src/pipeline` subsystem was verified in
isolation — receipt: `docs/tpot2-phase-shift-verification.receipt.json`
(`scripts/tpot2-harness-verify.sh`).

## 1. Validation of the existing TPOT2 surface — ADMITTED-quality

- Bounded statuses throughout; three-state law preserved in
  `pipeline::diagnostics_for_search` and `repair::simulate_admission` with
  explicit negative-control tests.
- Read-only surfaces; repair steps are intents routed through
  `CodeAction → clap-noun-verb → Receipt`.
- `pareto.rs` Pareto front: non-domination, determinism, irreflexivity invariants
  hold.
- Receipts are marker-style with 64-hex digests + boundary/checkpoint + a real
  negative control (different input → distinct digest).
- The `check-law-compliance.sh` scanner reports pre-existing hits only
  (`generate_axum_lsp.py`, `playground/ocel/*.jsonl`, matching `tower_lsp` inside
  `tower_lsp_max`); **none** in the TPOT2 additions — not a regression.

## 2. Findings and resolution

| # | Finding | Resolution | Status |
|---|---------|------------|--------|
| 1 | `search.rs` embedded `SystemTime::now()` nanoseconds in pipeline `id`s, so the serialized `best_pipeline.id` was not reproducible; the determinism witness projected the id out. | Ids are now PRNG-only (matching `pareto.rs`), PRNG consumption unchanged. Regression test `full_result_including_id_is_reproducible` pins full-result determinism, id included. | ADMITTED (harness) |
| 2 | `fitness::category_for` substring-matching disagreed with the authoritative `catalog::breed_category` (`markov_logic` → "logic" vs RuleBased). | `category_for` delegates to `breed_category`. Regression test `category_for_markov_logic_is_rule_not_logic`. | ADMITTED (harness) |
| 3 | The no-engine heuristic path ignored the OCEL log entirely. | New `src/pipeline/ocel.rs` grounds fitness in the log's OC-DFG; `auto_evaluator` selects it when a log is present. | ADMITTED (harness) |
| 4 | `PipelineSearchConfig` is unvalidated (`max < min` underflows the length range). | Documented; not yet guarded. | OPEN |

## 3. Phase-shift innovation (this change)

- `src/pipeline/phase.rs` — `ConformancePhase` (Frozen/Liquid/Vapor/Unsettled/
  Decomposed) ↔ bounded statuses, water→steam **1,700×** mesh-expansion factor,
  precedence mirroring the admission surface. See `docs/tpot2-phase-shift.md`.
- `src/pipeline/ocel.rs` — object-centric grounding. See
  `docs/tpot2-ocel-conformance.md`.
- `lsp-max-protocol/src/phase.rs` — read-only `max/phaseShift` surface +
  `PHASE-*` diagnostics, mirroring the verified `pipeline.rs` pattern.
- `docs/tpot2-autonomic-mesh.md` — layer-5 wiring of the expansion factor.
- `scripts/tpot2-harness-verify.sh`, `scripts/web-session-setup.sh` — web-session
  verification + SessionStart guidance (see
  `docs/claude-code-web-best-practices.md`).

## 4. Verification

`scripts/tpot2-harness-verify.sh`: 36 unit tests pass, `rustfmt --check` clean,
`cargo clippy -D warnings` clean, receipt bound and validated (ADMITTED) by
`scripts/validate-receipt-chain.sh`.

| Surface | Build/verify status |
|---|---|
| `src/pipeline/{search,fitness,ocel,phase,pareto,catalog,types}` | ADMITTED (standalone harness) |
| `lsp-max-protocol/src/phase.rs` (`max/phaseShift`) | CANDIDATE — build BLOCKED here (lsp-types-max absent); mirrors verified pattern |
| `max/phaseShift` runtime dispatch | OPEN |
| Full `just test` / `just dx-polish` | BLOCKED (siblings absent) |
| Engine-backed alignment conformance | UNKNOWN (needs wasm4pm) |

No victory language. OPEN/CANDIDATE/BLOCKED/UNKNOWN are not collapsed into
ADMITTED.
