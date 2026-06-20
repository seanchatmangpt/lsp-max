# Bayesian Network (bayesian_network)

| Field         | Value                          |
|---------------|-------------------------------|
| breed_id      | bayesian_network               |
| family        | ProbabilisticAI                |
| module        | bayesian_network               |
| struct        | BayesianNetwork                |
| paper         | pearl1988probabilistic         |
| oracle_value  | 0.284                          |
| status        | CANDIDATE                      |

## Algorithm Summary

Bayesian Networks represent joint probability distributions over sets of variables via
directed acyclic graphs, performing probabilistic inference through belief propagation
over conditional probability tables.

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

1. Add `expected_value: 0.284` to `tests/fixtures/papers/bayesian_network.json`
   (COG-005).
2. Implement belief propagation in `src/breeds/bayesian_network.rs`; confirm output
   equals 0.284 on the paper fixture (COG-006).
3. Produce `ocel/reports/bayesian_network.json` with provenance fields (COG-007).
4. Mirror fixture to
   `packages/cognition/src/__tests__/fixtures/papers/bayesian_network.json` (COG-009).
5. Audit production source for oracle fresh-name leakage (COG-010).
6. Set `admitted = true` in report when fitness = 1.0 (COG-011).

## Paper Reference

Pearl, J. (1988). *Probabilistic Reasoning in Intelligent Systems: Networks of Plausible
Inference*. Morgan Kaufmann, San Mateo, CA.
