# LSP-MAX-WIP-JIRA-v26.8.4

**Version:** v26.8.4 · **Last Updated:** 2026-08-04

## Purpose

This doc orients a cloud agent picking up unfinished work in this repository. It inventories
everything currently in flight — uncommitted work, failing tests, wiring gaps, and acknowledged
open items — with evidence (file:line, test name, or command output) for every claim. Status
values use this repo's bounded vocabulary (`ADMITTED`, `CANDIDATE`, `BLOCKED`, `REFUSED`,
`OPEN`, `PARTIAL`, `UNKNOWN`) per `CLAUDE.md` — no "done"/"complete"/"solved" language appears
here, and none should appear in follow-up work either.

This doc is self-contained: it does not require access to `~/.claude/plans/` (outside this
repo, not visible to a fresh cloud-agent session).

## How to use this doc

1. Read the ticket table below. Each row cites the evidence used to assign its status.
2. Before touching any ticket, re-run its **Verification command** and confirm the result
   matches what's recorded here — repo state may have moved since this audit.
3. Do not re-author work already covered by the **Do-not-duplicate** list.
4. Follow the repo's existing conventions: `..Default::default()` for struct literals, bounded
   status vocabulary, receipts (not stdout) as proof of admission, LSP surface stays read-only.

## Background: the in-flight ontology-correction effort

The bulk of current WIP is a checkpoint-based effort (CP0–CP9) to correct
`ontology/lsif06.ttl` so it accurately mirrors the real `Vertex`/`Edge`/`ItemEdgeProperty` enums
in `crates/lsp-max-lsif/src/lsif.rs` (24 vertex / 21 edge / 7 item-edge-property kinds), then
build SPARQL admission gates and generation targets on top of the corrected ontology, and apply
the same fidelity pattern to `ontology/lsp318.ttl`. The originating plan lives at
`~/.claude/plans/80-20-gall-test-refactor-cheerful-quokka.md` (outside this repo) — its
checkpoint definitions are summarized per-ticket below so this doc doesn't depend on that file.

## Ticket table

| ID | Title | Status | Evidence | Next action |
|----|-------|--------|----------|--------------|
| WIP-01 | CP0: LSIF baseline preflight | ADMITTED (verified) | `cargo test -p lsp-max-lsif` → 10/10 pass (`emitter_witness.rs` + `lsif_conformance_tests.rs`) | None — baseline confirmed clean before the ontology work began. |
| WIP-02 | CP1: `lsif06.ttl` fidelity correction | ADMITTED (verified) | `ontology/lsif06.ttl` (modified, +342 lines); `tests/lsif06_ontology_fidelity_proof.rs` (untracked, 229 lines, 5 tests) — all 5 pass. Hand-transcribes the real `#[serde(rename=...)]` variants from `crates/lsp-max-lsif/src/lsif.rs` and SPARQL-verifies the ontology matches exactly. | Commit once CP2–CP4 land together (see WIP-07 blocker first). |
| WIP-03 | CP2: SPARQL admission gates for `lsif06.ttl` | ADMITTED (verified) | `ontology/gates/{010_required,020_single_valued,030_value_constraints}.rq` (untracked) + `tests/lsif06_gates_and_sparql_derived_proof.rs` (untracked, 147 lines, 3 tests) — all pass, including a drift-injection-and-revert test at line 120 (`gate_030_refuses_a_deliberately_reintroduced_pre_cp1_bare_wire_label`) proving the gate is a real detector, not vacuous. | Commit alongside WIP-02. |
| WIP-04 | CP3: LSIF coverage report generation | PARTIAL | `ontology/lsif-report/{ggen.toml,LSIF_COVERAGE.md,queries/coverage.sparql,templates/coverage.md.tera,.ggen-v2/receipt.json}` exist and report all 45 vertex/edge kinds as "Constructed in source: yes" (`ontology/lsif-report/LSIF_COVERAGE.md:14`, dated 2026-08-04). But `ontology/lsif-report/ggen.toml:4` deliberately keeps this **out of** the root `gen.toml` so it "cannot risk the existing lsp318.ttl-driven generation rules." Root `gen.toml:8` `[ontology] source` is still only `ontology/lsp318.ttl` — confirmed by direct read. | Wire `lsif06.ttl` into the primary `gen.toml`/queries/templates pipeline once WIP-02/03 are committed and stable. Report's own caveat (`LSIF_COVERAGE.md:9-13`): "constructed in source" is not the same claim as "reachable from the real tree-sitter indexer walk at runtime" — that stronger claim is not yet checked. |
| WIP-05 | CP4: same fidelity/dual-proof pattern for `lsp318.ttl` | PARTIAL — 1 test failing | `ontology/lsp318.ttl` (modified) + `ontology/lsp-status-report/` (untracked) + `tests/lsp318_admitted_status_drift_proof.rs` (untracked, 391 lines, 5 tests) — **4/5 pass, 1 fails**. See WIP-07 for the failing test. | Fix WIP-07 first, then re-run full suite before considering this checkpoint stable. |
| WIP-06 | CP6 slice: stub-contract legibility (`expectsBinding`/`producesShape`) | ADMITTED (verified) | `queries/lsp-max/candidate_contracts.sparql` (untracked) + `tests/lsp318_candidate_contracts_are_queryable.rs` (untracked, not yet committed) — 4/4 tests pass. `lsp318.ttl` now carries `lsp:expectsBinding`/`lsp:producesShape` on CANDIDATE individuals (118 occurrences via grep). | Commit alongside WIP-02/03/05 once WIP-07 is resolved — this ticket's own additions are what broke WIP-07's fixture. |
| WIP-07 | **Failing test: CP4 drift-injection fixture is stale against CP6's ontology edits** | BLOCKED | `tests/lsp318_admitted_status_drift_proof.rs:346-390`, test `admitted_check_refuses_a_deliberately_injected_admitted_but_unregistered_method`, **fails deterministically**: `assertion left != right failed: the replacen target block must actually exist verbatim in lsp318.ttl for this injection to be real` (panic at line 359). Root cause: the test's hardcoded `replacen` target (lines 355-356) expects the `lsp:exit` block to end in `law:status law:CANDIDATE .` (period), but the real on-disk `ontology/lsp318.ttl:629-634` now reads `law:status law:CANDIDATE ;` (semicolon) because WIP-06's `lsp:expectsBinding "no-params"` was appended after this test was written. | One-line fix: update the test's fixture string at `tests/lsp318_admitted_status_drift_proof.rs:355-356` to match the current on-disk `lsp:exit` block shape (semicolon-continuation, trailing `expectsBinding` line). Not applied here per audit-only scope — this doc documents the fix, doesn't perform it. |
| WIP-08 | CP5: wire real handler bodies, flip CANDIDATE→ADMITTED | OPEN (not started) | `grep -c "law:ADMITTED" ontology/lsp318.ttl` → 0. No handler wiring against `crates/lsp-max-lsif/src/db.rs` query layer for `textDocument/{definition,references,hover,documentSymbol}` exists yet. | Requires WIP-05/07 resolved first (same ontology file). Scope: pick one method, wire it, flip its `law:status`, add an ADMITTED-transition proof test following the WIP-02/03 pattern. |
| WIP-09 | CP7/8/9: port pattern to sibling `ggen` repo (`crates/ggen-lsp`, frontmatter SHACL, ggen-mcp introspection) | OUT OF SCOPE for this repo | Plan items target `../ggen`, not `lsp-max`. No corresponding files exist in this repo. | Note only — do not attempt inside `lsp-max`. Track in the `ggen` repo's own WIP doc if one exists (UNKNOWN — not checked). |
| WIP-10 | Branch housekeeping | OPEN | `git branch -a` shows ~50 branches: ~30 `worktree-wf_*` (ephemeral worktree-session artifacts), several `subagent-*` orchestrator-preview branches, 20 `origin/claude/<adjective>-<name>-<hash>` cloud-agent session branches. Not individually audited — volume made a per-branch `git log master..<branch>` sweep out of scope for this pass. | A cloud agent (or a dedicated pass) should check each `origin/claude/*` and `worktree-wf_*` branch for unmerged unique commits before deleting; do not bulk-delete without that check. |
| WIP-11 | Local `master` behind `origin/master` | OPEN | `git rev-list --left-right --count master...origin/master` → `2  71` (local is 2 ahead, 71 behind). `origin/master` HEAD is `3c3e559` "Merge pull request #22 from seanchatmangpt/fix/wasm4pm-lsp-example-crates-io-dep". | Pull/rebase local `master` onto `origin/master` before starting new work to avoid divergence; reconcile the 2 local-only commits first (fix-forward per `CLAUDE.md` git workflow — no `git reset --hard`). |
| WIP-12 | Acknowledged OPEN items outside the LSIF06 effort | OPEN (repo-acknowledged, pre-existing) | `CHAIN-THEORY.md:69` (auto-discovery script `discover-lsp-chains.sh` absent), `:71` (no hot-reload of `lsp-max.toml`), `:79` (MCP server doc says `crates/lsp-max-mcp/` "not yet present" — but `crates/lsp-max-cli/src/bin/lsp-max-mcp.rs` exists as a binary; **possible doc drift, flagged UNKNOWN**, not verified further here), `:90` (no named subagent defs in `.claude/agents/`), `:122` (fitness scores logged via `tracing::debug!` but not wired into ANDON gate predicate); `ROADMAP.md:152,159-161` (Λ_CD RFC B per-server receipt chain, per-agent gate partitioning — `agent_scope` hardcoded `"global"` — and process-model mining, all `⬜ OPEN`); `crates/lsp-max-lsif/src/coverage.rs:112,159,201` (specific LSIF fields marked "OPEN substrate, not ADMITTED" — the pre-existing gap this whole WIP-01..07 effort is working toward closing, not yet reached). | These are pre-existing, already-tracked gaps — not newly discovered. Listed here so a cloud agent has one place to see them alongside the newer LSIF06 work. Resolve `CHAIN-THEORY.md:79` doc-drift question first (cheap to check, `UNKNOWN` status is unsatisfying). |

## Not a gap — intentionally deferred

`#[ignore]`d tests are gated by category, not abandoned:
- Perf/stress: `tests/test_m3_serialization_stress.rs`, `tests/test_compositor_perf_admission.rs`,
  `tests/test_challenger_m2_stress.rs`, `tests/test_perf_admission.rs` — run with `--include-ignored`.
- E2E (requires live server): `tests/e2e/test_t3_pairwise.rs`, `tests/e2e/test_f6_isolation.rs`,
  `tests/e2e/test_f7_static_graph.rs`.
- Long-running pre-publish: `crates/lsp-max-adapters/lsp-max-ast/src/codegen/tests/codegen.rs`
  (8 instances).

Codegen-emitted `// TODO: implement {}` stubs in `crates/playground/src/handlers/completions/mod.rs:83,87`
are template output written into *generated* code by a code generator — not repo TODOs.

## Do-not-duplicate list

- `docs/jira/v26.6.30/` — a complete ticket set (CC-001..007, `AGENT-PLAYBOOK.md`, `ARD-PRD.md`)
  for the Claude Code compositor/proxy/LSIF-tier work. Already implemented (see commit sequence
  `0eef70e`..`92c119e`). Different feature set from this doc — do not re-derive.
- `ROADMAP.md`, `CHAIN-THEORY.md`, `THESIS.md`, `DEFINITION_OF_DONE.md` — architecture/status
  docs. Neither mentions the LSIF06 Gall-checkpoint work by name (predates it) — WIP-01..08
  above are this doc's extension of that tracking, not a replacement for it.
- `~/.claude/plans/80-20-gall-test-refactor-cheerful-quokka.md` — the originating plan for
  WIP-01..09. Outside this repo; this doc's Background section and ticket rows are a
  self-contained summary of it for agents that can't read it directly.

## Verification commands

```sh
# WIP-01 baseline
cargo test -p lsp-max-lsif

# WIP-02/03 — lsif06.ttl fidelity + gates
cargo test --test lsif06_ontology_fidelity_proof
cargo test --test lsif06_gates_and_sparql_derived_proof

# WIP-05/07 — lsp318.ttl fidelity (will show 1 failure until WIP-07 is fixed)
cargo test --test lsp318_admitted_status_drift_proof

# WIP-06 — candidate contract queryability
cargo test --test lsp318_candidate_contracts_are_queryable

# WIP-08 — check ADMITTED method count (expect 0 until CP5 work lands)
grep -c "law:ADMITTED" ontology/lsp318.ttl

# WIP-04 — confirm gen.toml wiring state
sed -n '1,10p' gen.toml   # [ontology] source should still read lsp318.ttl only, pre-WIP-04

# WIP-11 — branch/remote drift
git rev-list --left-right --count master...origin/master
```

## See Also

- `~/.claude/plans/80-20-gall-test-refactor-cheerful-quokka.md` — originating plan (outside repo, summarized above)
- `ROADMAP.md` — broader project roadmap, OPEN items cited in WIP-12
- `CHAIN-THEORY.md` — chain-discovery architecture, OPEN items cited in WIP-12
- `docs/jira/v26.6.30/README.md` — prior, completed ticket set (different feature area)
- `CLAUDE.md` — bounded-status vocabulary, git workflow (fix-forward only), verification discipline
