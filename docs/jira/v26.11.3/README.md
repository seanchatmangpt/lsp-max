# v26.11.3

Wave index for lsp-max v26.11.3 (compositor hardening wave). Receipts live in
`receipts/`; this file is the wave index. Standing vocabulary:
ALIVE / PARTIAL / BLOCKED / UNVERIFIED / REFUSED. Never round up.

Exit record law: A gate is witnessed by an exit record: either (a) the process exit status, or (b) cargo's "N passed; 0 failed" summary line. A passed-count without a failed-count is not an exit record.

## Lane table

Standing words re-read from each receipt body on disk (2026-11-03,
cl-close lane), never from memory.

| Lane | Receipt | Standing | Gate (exit record) |
|---|---|---|---|
| CL-F | receipts/cl-f-eviction.md | PARTIAL | core lane ADMITTED; full-suite sweep BLOCKED on one pre-existing environment-missing external binary |
| CL-G | receipts/cl-g-priority.md | PARTIAL_ALIVE | owned tests 10/10 + 5/5; full-suite regression compile-aborted on a sibling lane's in-flight test target (compile abort, not a failure count) |
| CL-H | receipts/cl-h-end-position.md | PARTIAL | diagnostic_end_position_test 4 passed / 0 failed, exit 0; residual 1 CLOSED; open suspected regression in diagnostic_relay_probe attributed to CL-E/F/G in-flight work |
| CL-N | receipts/cl-n-flake.md | PARTIAL | crash_isolation court green under load (3/3 in isolation); one open pin added |
| CL-E | receipts/cl-e-server-handle.md | ALIVE | dead_child_fails_typed "ok. 3 passed; 0 failed" exit 0; server-handle suites "ok. 9 passed; 0 failed" exit 0; live-silent timeout court 3/0 exit 0 (30.02 s include-ignored) |
| LM-1 | receipts/lm-1.md | ADMITTED (lane) / PARTIAL (suite) | registry_ontology_test "6 passed; 0 failed" exit 0; compositor --lib "52 passed; 0 failed"; integration 47 passed; test_gate_path_byte_equal_court "4 passed; 0 failed" |
| SA-10 | receipts/sa-10.md | PARTIAL (per receipt body) | registry_servers.rq header + determinism rule ADMITTED ("6 passed / 0 failed" incl. determinism_double_run_byte_identical); full-manifest `ggen sync run` BLOCKED on a pre-existing cc-chicago-tests failure, not introduced by SA-10 |
| SA-11 | receipts/sa-11.md | PARTIAL_ALIVE | scaffold gate over staged copy exit 0; "4 passed; 0 failed" exit 0; `cargo check -p lsp-max-scaffold` Finished exit 0 |
| XG6-GATE-PATH-COURT | receipts/xg6-gate-path-court.md | ADMITTED (court) / OPEN (widening) | `cargo test --test test_gate_path_byte_equal_court` → "4 passed; 0 failed" (exit 0) |
| XG6-LSP-TYPES-MAX-PIN | receipts/xg6-lsp-types-max-pin.md | ADMITTED (pin decision) / UNVERIFIED beyond crates.io | pin decision + 4 manifest sites re-read from disk 2026-11-03; upstream 26.11.3 publication UNKNOWN |

## In-flight-expected (law 5, cl-close-2)

No lsp-max receipts were outstanding at this pass (2026-11-03); DS-7/DS-8,
GA-2/GA-6 and PG-A in-flight expectations are tracked in ggen's README
new-wave section.

## Open residuals

- CL-F: full-suite sweep blocked on one environment-missing external binary
  (pre-existing, not session-introduced).
- CL-G: full-suite regression never witnessed green in one command —
  sibling-lane compile abort, not a failure count.
- CL-H: suspected regression in `diagnostic_relay_probe`
  (dispatch-level, CL-E/F/G-owned files) — OPEN, not attributed to CL-H's diff.
- CL-N: the crash_isolation pin superseded by CL-G's fix lane was flipped;
  one open pin added by the flake court. The "load flake" root cause was
  test/src drift against landed M8 work, not a timing race.
- Full-repo single-command green full-suite run: not witnessed this wave.
