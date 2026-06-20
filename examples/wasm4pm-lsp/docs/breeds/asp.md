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

Answer Set Programming computes stable models of logic programs under the stable-model
semantics, deriving all consequences entailed by a set of rules and constraints via
negation-as-failure.

## COG Law Status

| Law     | Status  |
|---------|---------|
| COG-001 | PARTIAL |
| COG-002 | PARTIAL |
| COG-003 | PARTIAL |
| COG-004 | PARTIAL |
| COG-005 | OPEN    |
| COG-006 | OPEN    |
| COG-007 | OPEN    |
| COG-008 | OPEN    |
| COG-009 | OPEN    |
| COG-010 | OPEN    |
| COG-011 | OPEN    |
| COG-012 | PARTIAL |

## Admission Path

1. Add `expected_value` to `tests/fixtures/papers/asp.json` (COG-005).
2. Implement stable-model computation in `src/breeds/asp.rs` and verify output matches
   oracle_value = 0.0 on the paper fixture (COG-006).
3. Run OCEL fitness check; update `ocel/reports/asp.json` with `measured_by`,
   `measured_on`, and `run_id` fields (COG-007).
4. Add TS fixture mirror at
   `packages/cognition/src/__tests__/fixtures/papers/asp.json` (COG-009).
5. Audit production source for oracle fresh-name leakage; remove if present (COG-010).
6. Set `admitted = true` in `ocel/reports/asp.json` when fitness = 1.0 (COG-011).

## Paper Reference

Gelfond, M., & Lifschitz, V. (1991). Classical negation in logic programs and disjunctive
databases. *New Generation Computing*, 9(3-4), 365-385.
