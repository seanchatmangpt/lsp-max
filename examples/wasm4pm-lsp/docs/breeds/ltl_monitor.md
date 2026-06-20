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

LTL Runtime Monitoring evaluates Linear Temporal Logic formulae over finite execution
traces, classifying each prefix as permanently true, permanently false, or currently
inconclusive against the property specification.

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

1. Add `expected_value: 1.0` to `tests/fixtures/papers/ltl_monitor.json` (COG-005).
2. Implement LTL trace evaluation in `src/breeds/ltl_monitor.rs`; confirm oracle
   output = 1.0 (COG-006).
3. Produce OCEL fitness report at `ocel/reports/ltl_monitor.json` with provenance
   fields (COG-007).
4. Mirror fixture to
   `packages/cognition/src/__tests__/fixtures/papers/ltl_monitor.json` (COG-009).
5. Audit production source for oracle fresh-name leakage (COG-010).
6. Set `admitted = true` in report when fitness = 1.0 (COG-011).

## Paper Reference

Bauer, A., Leucker, M., & Schallhart, C. (2011). Runtime verification for LTL and TLTL.
*ACM Transactions on Software Engineering and Methodology*, 20(4), Article 14.
