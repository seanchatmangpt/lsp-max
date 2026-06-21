# Case-Based Reasoner (cbr)

| Field         | Value                   |
|---------------|------------------------|
| breed_id      | cbr                     |
| family        | SymbolicAI              |
| module        | cbr                     |
| struct        | Cbr                     |
| paper         | kolodner1993cbr         |
| oracle_value  | 0.85                    |
| status        | CANDIDATE               |

## Algorithm Summary

Kolodner (1993) case-based reasoning retrieve phase. Extracts numeric feature vectors
from cases and query, applies weighted cosine similarity Σ(w·q·c) / (√Σ(w·q²) ·
√Σ(w·c²)), ranks all cases by similarity, returns the top-ranked case's id, solution,
and similarity score.

## COG Law Status

| Law     | Status    | Reason                                                              |
|---------|-----------|---------------------------------------------------------------------|
| COG-001 | ADMITTED  | breed module exists with real implementation                        |
| COG-002 | CANDIDATE | OCPN model stub upgraded to breed-specific flow                     |
| COG-003 | OPEN      | fitness report has fitness=0.0, no conformance runner yet           |
| COG-004 | ADMITTED  | paper fixture file exists with real inputs                          |
| COG-005 | ADMITTED  | fixture has real expected.value (not PENDING)                       |
| COG-006 | OPEN      | fitness ≠ 1.0 — requires conformance suite measurement              |
| COG-007 | OPEN      | provenance fields need measured_by from conformance runner          |
| COG-008 | ADMITTED  | this doc card exists                                                |
| COG-009 | ADMITTED  | TS fixture mirror exists in packages/                               |
| COG-010 | CANDIDATE | no non-comment oracle literals found; scan test at tests/cog010_oracle_scan.rs |
| COG-011 | OPEN      | DoD incomplete: fitness=0.0, admitted=false in report               |
| COG-012 | ADMITTED  | dispatch arm present in dispatch.rs                                 |

## Admission Path

The following laws remain OPEN or CANDIDATE and block full admission:

- **COG-003**: No conformance runner has executed against this breed. Fitness remains 0.0
  until `wasm4pm-conformance-runner` produces a measured result.
- **COG-006**: Conformance suite measurement required; fitness ≠ 1.0.
- **COG-007**: `measured_by`, `measured_on`, and `run_id` in `ocel/reports/cbr.json` are
  OPEN placeholders; a conformance runner execution must populate them.
- **COG-010**: No non-comment oracle literals detected in breed source. Formal scan test
  at `tests/cog010_oracle_scan.rs` covers this breed; receipt written to
  `tests/receipts/cog010-scan.json` at runtime. Status remains CANDIDATE until the test
  has been run and its receipt is present on disk.
- **COG-011**: `admitted` stays false and `fitness` stays 0.0 until COG-003/006/007
  are resolved by a conformance runner run.

## Paper Reference

Kolodner, J. (1993). *Case-Based Reasoning*. Morgan Kaufmann, San Mateo, CA.
