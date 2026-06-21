# Frame Inheritance System (frames_inheritance)

| Field         | Value                     |
|---------------|--------------------------|
| breed_id      | frames_inheritance        |
| family        | SymbolicAI                |
| module        | frames_inheritance        |
| struct        | FramesInheritance         |
| paper         | minsky1975frames          |
| oracle_value  | 0.0                       |
| status        | CANDIDATE                 |

## Algorithm Summary

Minsky (1975) frame inheritance system. Resolves a slot query on a named frame by
walking the `isa` chain upward until the slot is found or the chain is exhausted. Returns
the slot value, the frame where it was found (inherited_from), and the ISA-chain depth.
Detects cycles up to MAX_DEPTH=20.

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
| COG-010 | CANDIDATE | no oracle fresh-name leak detected (pending formal scan)            |
| COG-011 | OPEN      | DoD incomplete: fitness=0.0, admitted=false in report               |
| COG-012 | ADMITTED  | dispatch arm present in dispatch.rs                                 |

## Admission Path

The following laws remain OPEN or CANDIDATE and block full admission:

- **COG-003**: No conformance runner has executed against this breed. Fitness remains 0.0
  until `wasm4pm-conformance-runner` produces a measured result.
- **COG-006**: Conformance suite measurement required; fitness ≠ 1.0.
- **COG-007**: `measured_by`, `measured_on`, and `run_id` in
  `ocel/reports/frames_inheritance.json` are OPEN placeholders; a conformance runner
  execution must populate them.
- **COG-010**: No oracle fresh-name leak detected in manual review, but a formal scan
  has not been recorded. Status remains CANDIDATE until scan receipt exists.
- **COG-011**: `admitted` stays false and `fitness` stays 0.0 until COG-003/006/007
  are resolved by a conformance runner run.

## Paper Reference

Minsky, M. (1975). A framework for representing knowledge. In P. H. Winston (Ed.),
*The Psychology of Computer Vision* (pp. 211-277). McGraw-Hill, New York.
