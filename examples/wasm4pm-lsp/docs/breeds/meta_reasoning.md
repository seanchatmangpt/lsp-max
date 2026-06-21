# Meta Reasoner (meta_reasoning)

| Field         | Value                     |
|---------------|--------------------------|
| breed_id      | meta_reasoning            |
| family        | MetaCognition             |
| module        | meta_reasoning            |
| struct        | MetaReasoning             |
| paper         | cox2005metacognition      |
| oracle_value  | 0.0                       |
| status        | CANDIDATE                 |

## Algorithm Summary

Cox (2005) meta-cognitive strategy selector. Introspects cognitive state (confidence,
time_elapsed) and resource constraints (time_budget, available_knowledge). Filters
strategies by knowledge and time feasibility. Scores each feasible strategy as
utility/√cost × (1 + (1−confidence)) and selects the highest-ranked strategy.

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
| COG-010 | CANDIDATE | no oracle fresh-name leak detected (pending formal scan)            |
| COG-011 | OPEN      | DoD incomplete: fitness=0.0, admitted=false in report               |
| COG-012 | ADMITTED  | dispatch arm present in dispatch.rs                                 |

## Admission Path

The following laws remain OPEN or CANDIDATE and block full admission:

- **COG-003**: No conformance runner has executed against this breed. Fitness remains 0.0
  until `wasm4pm-conformance-runner` produces a measured result.
- **COG-006**: Conformance suite measurement required; fitness ≠ 1.0.
- **COG-007**: `measured_by`, `measured_on`, and `run_id` in
  `ocel/reports/meta_reasoning.json` are OPEN placeholders; a conformance runner
  execution must populate them.
- **COG-010**: No oracle fresh-name leak detected in manual review, but a formal scan
  has not been recorded. Status remains CANDIDATE until scan receipt exists.
- **COG-011**: `admitted` stays false and `fitness` stays 0.0 until COG-003/006/007
  are resolved by a conformance runner run.

## Paper Reference

Cox, M. T. (2005). Metacognition in computation: A selected research review.
*Artificial Intelligence*, 169(2), 104-141.
