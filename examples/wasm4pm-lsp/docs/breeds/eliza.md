# ELIZA Frame Reasoner (eliza)

| Field         | Value                    |
|---------------|-------------------------|
| breed_id      | eliza                    |
| family        | SymbolicAI               |
| module        | frame                    |
| struct        | Eliza                    |
| paper         | weizenbaum1966eliza      |
| oracle_value  | 0.0                      |
| status        | CANDIDATE                |

## Algorithm Summary

Weizenbaum (1966) ELIZA script-based conversational agent. Matches each input utterance
against a list of `*`-wildcard patterns in priority order. On match, extracts capture
groups and fills the next cyclic response template using `(1)`, `(2)`, … substitution.
Falls back to scripted responses if no pattern matches.

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
- **COG-007**: `measured_by`, `measured_on`, and `run_id` in `ocel/reports/eliza.json`
  are OPEN placeholders; a conformance runner execution must populate them.
- **COG-010**: No non-comment oracle literals detected in `src/breeds/frame.rs`. Formal
  scan test at `tests/cog010_oracle_scan.rs` exists and writes a receipt at runtime.
  Status remains CANDIDATE until the test has been run and its receipt is present on disk.
- **COG-011**: `admitted` stays false and `fitness` stays 0.0 until COG-003/006/007
  are resolved by a conformance runner run.

## Paper Reference

Weizenbaum, J. (1966). ELIZA — A computer program for the study of natural language
communication between man and machine. *Communications of the ACM*, 9(1), 36-45.
