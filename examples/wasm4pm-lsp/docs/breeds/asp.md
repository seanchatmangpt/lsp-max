# Answer Set Programmer (asp)

| Field         | Value                        |
|---------------|------------------------------|
| breed_id      | asp                          |
| family        | FormalMethods                |
| module        | asp                          |
| struct        | Asp                          |
| paper         | gelfond1991stable            |
| oracle_value  | 0.0                          |
| status        | CANDIDATE                    |

## Algorithm Summary

Answer Set Programming via Gelfond-Lifschitz (1991) stable model semantics. Constructs
the GL reduct of a logic program relative to a candidate model, then finds the minimal
Herbrand model via forward-chaining fixpoint. If the minimal model equals the candidate,
it is a stable model. Enumerates all 2^|Herbrand base| subsets.

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
- **COG-007**: `measured_by`, `measured_on`, and `run_id` in `ocel/reports/asp.json` are
  OPEN placeholders; a conformance runner execution must populate them.
- **COG-010**: No non-comment oracle literals detected in breed source. Formal scan test
  at `tests/cog010_oracle_scan.rs` exists and writes a receipt at runtime. Status remains
  CANDIDATE until the test has been run and its receipt is present on disk.
- **COG-011**: `admitted` stays false and `fitness` stays 0.0 until COG-003/006/007
  are resolved by a conformance runner run.

## Paper Reference

Gelfond, M., & Lifschitz, V. (1991). Classical negation in logic programs and disjunctive
databases. *New Generation Computing*, 9(3-4), 365-385.
