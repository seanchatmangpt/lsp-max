# LTL Runtime Monitor (ltl_monitor)

| Field         | Value                   |
|---------------|------------------------|
| breed_id      | ltl_monitor             |
| family        | FormalMethods           |
| module        | ltl_monitor             |
| struct        | LtlMonitor              |
| paper         | bauer2011ltl            |
| oracle_value  | 1.0                     |
| status        | CANDIDATE               |

## Algorithm Summary

Bauer et al. (2011) runtime monitor for finite-trace LTL. Parses a formula string into
an AST (G, F, X, U, →, ∧, ∨, ¬, atom). Evaluates the formula recursively over a JSON
event trace using finite-trace semantics: G folds all positions, F scans for any true,
Until scans for a ψ-witness with φ holding in the prefix, X returns false at the last
position.

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
- **COG-007**: `measured_by`, `measured_on`, and `run_id` in
  `ocel/reports/ltl_monitor.json` are OPEN placeholders; a conformance runner execution
  must populate them.
- **COG-010**: No non-comment oracle literals detected in breed source. Formal scan test
  at `tests/cog010_oracle_scan.rs` exists and writes a receipt at runtime. Status remains
  CANDIDATE until the test has been run and its receipt is present on disk.
- **COG-011**: `admitted` stays false and `fitness` stays 0.0 until COG-003/006/007
  are resolved by a conformance runner run.

## Paper Reference

Bauer, A., Leucker, M., & Schallhart, C. (2011). Runtime verification for LTL and TLTL.
*ACM Transactions on Software Engineering and Methodology*, 20(4), Article 14.
