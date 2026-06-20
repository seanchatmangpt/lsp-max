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

Meta-Reasoning monitors and regulates object-level reasoning processes, detecting
failures or resource overruns and selecting corrective strategies by reasoning about the
system's own knowledge state and inference methods.

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

1. Add `expected_value: 0.0` to
   `tests/fixtures/papers/meta_reasoning.json` (COG-005).
2. Implement introspective monitoring loop in `src/breeds/meta_reasoning.rs`; confirm
   oracle output = 0.0 (COG-006).
3. Produce OCEL fitness report at `ocel/reports/meta_reasoning.json` with provenance
   (COG-007).
4. Mirror fixture to
   `packages/cognition/src/__tests__/fixtures/papers/meta_reasoning.json` (COG-009).
5. Audit production source for oracle fresh-name leakage (COG-010).
6. Set `admitted = true` in report when fitness = 1.0 (COG-011).

## Paper Reference

Cox, M. T. (2005). Metacognition in computation: A selected research review.
*Artificial Intelligence*, 169(2), 104-141.
