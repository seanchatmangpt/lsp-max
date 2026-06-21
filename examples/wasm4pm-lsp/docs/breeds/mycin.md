# MYCIN Production Rules (mycin)

| Field         | Value                     |
|---------------|--------------------------|
| breed_id      | mycin                     |
| family        | SymbolicAI                |
| module        | production_rules          |
| struct        | Mycin                     |
| paper         | shortliffe1976mycin       |
| oracle_value  | 0.693                     |
| status        | CANDIDATE                 |

## Algorithm Summary

Shortliffe (1976) MYCIN certainty factor calculus. Fires all production rules whose
hypothesis matches the query and whose evidence is in the active evidence set. Combines
fired CFs pairwise using: CF_new = CF1 + CF2·(1−CF1) for positive CFs,
CF1 + CF2·(1+CF1) for negative CFs, (CF1+CF2)/(1−min(|CF1|,|CF2|)) for mixed signs.
Returns the accumulated CF as the conclusion confidence.

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
- **COG-007**: `measured_by`, `measured_on`, and `run_id` in `ocel/reports/mycin.json`
  are OPEN placeholders; a conformance runner execution must populate them.
- **COG-010**: No oracle fresh-name leak detected in `src/breeds/production_rules.rs` in
  manual review, but a formal scan has not been recorded. Status remains CANDIDATE until
  scan receipt exists.
- **COG-011**: `admitted` stays false and `fitness` stays 0.0 until COG-003/006/007
  are resolved by a conformance runner run.

## Paper Reference

Shortliffe, E. H. (1976). *Computer-Based Medical Consultations: MYCIN*. American
Elsevier, New York.
