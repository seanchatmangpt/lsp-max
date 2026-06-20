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

Case-Based Reasoning retrieves the most similar stored case to a new problem, adapts that
case's solution, applies it, and retains the outcome to grow the case library over time.

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

1. Add `expected_value: 0.85` to `tests/fixtures/papers/cbr.json` (COG-005).
2. Implement retrieve-reuse-revise-retain cycle in `src/breeds/cbr.rs`; confirm oracle
   output = 0.85 (COG-006).
3. Produce OCEL fitness report at `ocel/reports/cbr.json` with full provenance
   (COG-007).
4. Mirror fixture to
   `packages/cognition/src/__tests__/fixtures/papers/cbr.json` (COG-009).
5. Audit production source for oracle fresh-name leakage (COG-010).
6. Set `admitted = true` in report when fitness = 1.0 (COG-011).

## Paper Reference

Kolodner, J. (1993). *Case-Based Reasoning*. Morgan Kaufmann, San Mateo, CA.
