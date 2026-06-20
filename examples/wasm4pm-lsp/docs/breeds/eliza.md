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

ELIZA applies pattern-matching scripts (frames) to transform user input into scripted
responses, using decomposition rules and reassembly templates without any semantic
understanding of the content.

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

1. Add `expected_value: 0.0` to `tests/fixtures/papers/eliza.json` (COG-005).
2. Implement DOCTOR-script frame matching in `src/breeds/frame.rs`; confirm oracle
   output = 0.0 (COG-006).
3. Produce OCEL fitness report at `ocel/reports/eliza.json` with provenance (COG-007).
4. Mirror fixture to
   `packages/cognition/src/__tests__/fixtures/papers/eliza.json` (COG-009).
5. Audit `src/breeds/frame.rs` for oracle fresh-name leakage (COG-010).
6. Set `admitted = true` in report when fitness = 1.0 (COG-011).

## Paper Reference

Weizenbaum, J. (1966). ELIZA — A computer program for the study of natural language
communication between man and machine. *Communications of the ACM*, 9(1), 36-45.
