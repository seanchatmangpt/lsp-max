# POMDP Solver (pomdp)

| Field         | Value                       |
|---------------|----------------------------|
| breed_id      | pomdp                       |
| family        | ReinforcementLearning       |
| module        | pomdp                       |
| struct        | Pomdp                       |
| paper         | kaelbling1998planning       |
| oracle_value  | 0.969                       |
| status        | CANDIDATE                   |

## Algorithm Summary

Kaelbling et al. (1998) Tiger POMDP solved by finite-horizon state-space value
iteration. Hardwired to the Tiger Problem (S={tiger-left,tiger-right},
A={listen,open-left,open-right}, γ=0.95). Runs N Bellman backup iterations, then
projects the value function to the uniform belief [0.5, 0.5] and picks the greedy action.

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
- **COG-007**: `measured_by`, `measured_on`, and `run_id` in `ocel/reports/pomdp.json`
  are OPEN placeholders; a conformance runner execution must populate them.
- **COG-010**: No non-comment oracle literals detected in breed source. Formal scan test
  at `tests/cog010_oracle_scan.rs` covers this breed; receipt written to
  `tests/receipts/cog010-scan.json` at runtime. Status remains CANDIDATE until the test
  has been run and its receipt is present on disk.
- **COG-011**: `admitted` stays false and `fitness` stays 0.0 until COG-003/006/007
  are resolved by a conformance runner run.

## Paper Reference

Kaelbling, L. P., Littman, M. L., & Cassandra, A. R. (1998). Planning and acting in
partially observable stochastic domains. *Artificial Intelligence*, 101(1-2), 99-134.
