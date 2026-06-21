# Large Language Model Reasoning (llm)

| Field         | Value                          |
|---------------|-------------------------------|
| breed_id      | llm                            |
| family        | NeuralAI                       |
| module        | llm                            |
| struct        | Llm                            |
| paper         | brown2020language              |
| oracle_value  | 0.0 (non-deterministic)        |
| status        | CANDIDATE                      |

## Algorithm Summary

Brown et al. (2020) GPT-3 large language model: sends a user prompt via the
Anthropic Messages API (`POST /v1/messages`) using a blocking HTTP call wrapped
in a dedicated thread to avoid blocking the tokio async runtime. Returns the
response text, model name, and token counts. Gracefully returns `None` when
`ANTHROPIC_API_KEY` is absent, allowing CI runs without credentials to skip
rather than fail.

Pass criterion: response field is a non-empty string. The output is
non-deterministic by design, so no fixed oracle value applies.

## COG Law Status

| Law     | Status    | Reason                                                              |
|---------|-----------|---------------------------------------------------------------------|
| COG-001 | ADMITTED  | breed module exists with real implementation                        |
| COG-002 | CANDIDATE | OCPN model exists with 6 places and 4 transitions                  |
| COG-003 | OPEN      | fitness report has fitness=0.0, no conformance runner run yet       |
| COG-004 | ADMITTED  | paper fixture file exists with real inputs                          |
| COG-005 | ADMITTED  | fixture has real expected.value (not PENDING)                       |
| COG-006 | OPEN      | fitness ≠ 1.0 — requires conformance suite measurement              |
| COG-007 | OPEN      | provenance fields need measured_by from conformance runner          |
| COG-008 | ADMITTED  | this doc card exists                                                |
| COG-009 | ADMITTED  | TS fixture mirror exists in packages/                               |
| COG-010 | CANDIDATE | no oracle literal to scan (oracle_value=0.0, excluded from scan)   |
| COG-011 | OPEN      | DoD incomplete: fitness=0.0, admitted=false in report               |
| COG-012 | ADMITTED  | dispatch arm present in dispatch.rs                                 |

## Admission Path

The following laws remain OPEN or CANDIDATE and block full admission:

- **COG-003**: No conformance runner has executed against this breed. Fitness
  remains 0.0 until `wasm4pm-conformance-runner` produces a measured result.
  Note: if `ANTHROPIC_API_KEY` is absent, the breed skips rather than fails.
- **COG-006**: Conformance suite measurement required; fitness ≠ 1.0.
- **COG-007**: `measured_by`, `measured_on`, and `run_id` in
  `ocel/reports/llm.json` are OPEN placeholders; a conformance runner
  execution must populate them.
- **COG-010**: oracle_value is 0.0 (too common to scan meaningfully);
  no oracle injection risk — remains CANDIDATE pending formal scan policy.
- **COG-011**: `admitted` stays false and `fitness` stays 0.0 until COG-003/006/007
  are resolved by a conformance runner run.

## Paper Reference

Brown, T., Mann, B., Ryder, N., et al. (2020). *Language Models are Few-Shot
Learners*. Advances in Neural Information Processing Systems (NeurIPS), 33,
1877–1901.
