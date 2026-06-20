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

MYCIN-style production rule systems chain IF-THEN rules with certainty factors, using
backward chaining to diagnose hypotheses and combining evidence across rules via a
certainty factor algebra.

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

1. Add `expected_value: 0.693` to `tests/fixtures/papers/mycin.json` (COG-005).
2. Implement certainty-factor backward chaining in `src/breeds/production_rules.rs`;
   confirm oracle output = 0.693 (COG-006).
3. Produce OCEL fitness report at `ocel/reports/mycin.json` with full provenance
   (COG-007).
4. Mirror fixture to
   `packages/cognition/src/__tests__/fixtures/papers/mycin.json` (COG-009).
5. Audit `src/breeds/production_rules.rs` for oracle fresh-name leakage (COG-010).
6. Set `admitted = true` in report when fitness = 1.0 (COG-011).

## Paper Reference

Shortliffe, E. H. (1976). *Computer-Based Medical Consultations: MYCIN*. American
Elsevier, New York.
