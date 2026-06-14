# docs/archive

Point-in-time documents retained for provenance. **These make no current claim.**
Each was accurate at its checkpoint; live status is carried by the top-level
`ROADMAP.md`, `AGENTS.md`, `CHANGELOG.md`, and the living docs under `docs/`
(`law/`, `adr/`, `research/`, and the theses/explorations in `docs/reports/`).

Nothing here should be cited as the current state of the runtime. If an archived
report and a live doc disagree, the live doc governs; the archived report records
only what was observed at its checkpoint.

## Contents

### `max-001-rounds/`
The first multi-agent ("1000x") rounds against the lsp-max surface.

- `conformance/` — per-phase conformance snapshots `MAX-001`..`MAX-007`, plus
  `SPECGEN-001-bootstrap-report.md`. Each is a checkpoint of admitted/refused/unknown
  axes at the time of that round; superseded by the current `ROADMAP.md` and the
  Λ_CD predicate audit in `AGENTS.md`.
- `agents-reports/` — individual agent delivery/analysis reports from MAX-001.
- `evidence/` — `GC005_HANDOFF.md` and `GC005_PROCESS_EVIDENCE.jsonl`: the raw
  OCEL evidence and handoff for the GC005 authority-surface checkpoint.

### `v26.6.5/`
The v26.6.5 PRD/ARD release bundle (Oxigraph/SPARQL admitted graph control plane).
Version-pinned to a release prior to the current workspace version (CalVer 26.6.9).
The operative design record for that direction now lives in
`docs/reports/ARD-OXIGRAPH-SPARQL.md` and `docs/reports/oxigraph_store_exploration.md`,
with current status tracked as OPEN/CANDIDATE in `ROADMAP.md`.
