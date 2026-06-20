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

Frame-based inheritance organises knowledge into structured slot-filler records arranged
in an ISA hierarchy, propagating default values and procedural attachments from parent
frames to specialised child frames.

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
   `tests/fixtures/papers/frames_inheritance.json` (COG-005).
2. Implement slot inheritance traversal in `src/breeds/frames_inheritance.rs`; confirm
   oracle output = 0.0 (COG-006).
3. Produce OCEL fitness report at `ocel/reports/frames_inheritance.json` with full
   provenance (COG-007).
4. Mirror fixture to
   `packages/cognition/src/__tests__/fixtures/papers/frames_inheritance.json` (COG-009).
5. Audit production source for oracle fresh-name leakage (COG-010).
6. Set `admitted = true` in report when fitness = 1.0 (COG-011).

## Paper Reference

Minsky, M. (1975). A framework for representing knowledge. In P. H. Winston (Ed.),
*The Psychology of Computer Vision* (pp. 211-277). McGraw-Hill, New York.
