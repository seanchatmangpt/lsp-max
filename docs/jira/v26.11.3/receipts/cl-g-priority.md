# CL-G Receipt — Priority-string validation (M9 hazard fix)

Lane: IMPLEMENTATION (CL-G)
Date: 2026-11-03
Status: PARTIAL_ALIVE — owned tests green (10/10 + 5/5, commands cited); full-suite
regression run compiled-and-passed everything except a sibling lane's in-flight
test target (compile abort, not a failure count).

## Subject (files touched)

- `/Users/sac/lsp-max/crates/lsp-max-compositor/src/config.rs` (owned)
- `/Users/sac/lsp-max/crates/lsp-max-compositor/tests/priority_validation_test.rs` (owned, new)
- `/Users/sac/lsp-max/crates/lsp-max-compositor/src/registry.rs` — MINIMAL unblock, disclosed
  (see "Ownership crossings"): `from_config` now maps through
  `ChildTier::from_priority` so the two paths agree on every accepted string
  (contract item 2 was impossible without it: `secondary`/`lsif` diverged).
- `/Users/sac/lsp-max/crates/lsp-max-compositor/tests/priority_semantics.rs` — MINIMAL edit, disclosed: flipped
  M9's hazard-pin test 10 from pinning the escalation to pinning the FIX
  (`typo_priority_is_refused_at_load_and_from_config_matches_from_priority`).
  The pin existed to be superseded by the fix lane.
- `/Users/sac/lsp-max/crates/lsp-max-compositor/tests/andon_prefixes.rs` — MINIMAL unblock, disclosed: fixture
  used `priority = "primary"`, which is outside the contract's accepted set; swapped
  to `"full"` (identical tier behavior: both map Primary; from_config gave Primary
  for "primary", from_priority("full") = Primary).

## The fix

1. **Load-time refusal** (config.rs): `ServerEntry.priority` now deserializes through
   `deserialize_validated_priority` — exact-string membership in
   `ACCEPTED_PRIORITY_STRINGS = ["full","diagnostics-only","semantic","secondary","lsif"]`.
   Any other string → `InvalidPriorityError` (Display: `invalid priority "ful": accepted
   values are exactly [full, diagnostics-only, semantic, secondary, lsif]`), propagated
   through `CompositorConfig::from_toml_file`'s `Box<dyn Error>`.
2. **Parity** (registry.rs from_config): primary-tier now computed via
   `ChildTier::from_priority` (same mapping the spawn path in server.rs already used).
   Secondary-list tier: DiagnosticsOnly/Lsif keep their tier on both slots; Primary
   strings get Secondary on the secondary list (preserves M9 test 4's
   full→Primary/Secondary semantics). Typos reaching from_config by direct struct
   construction demote to DiagnosticsOnly — never escalate.
3. **Valid set unchanged for production manifests**: both production manifests
   (`/Users/sac/lsp-max/lsp-max.toml`, `/Users/sac/claude-code-config-lsp/lsp-max.toml`)
   use only "full"/"diagnostics-only" — both parse unchanged.

## Verification

```
cd /Users/sac/lsp-max && cargo test -p lsp-max-compositor --test priority_validation_test --test priority_semantics
priority_semantics:        10 passed; 0 failed (exit 0)
priority_validation_test:   5 passed; 0 failed (exit 0)
```

M9 test 10 flipped: was `from_config_diverges_from_from_priority_for_full_string`
(pin on the escalation hazard), now
`typo_priority_is_refused_at_load_and_from_config_matches_from_priority` (pin on
the fixed behavior: typed load-refusal + tier parity).

Full compositor suite (excluding sibling in-flight `child_eviction_test`, compile abort
E0308/E0425 — sibling CL-E/CL-F surface, census law: compile-abort ≠ failure count):

```
andon_prefixes 11 ok · cc_007_lsif_routing 8 ok · crash_isolation 3 ok ·
diagnostic_end_position_test 4 ok · diagnostics_merge 9 ok · e2e 2 ok ·
fanout_latency 1 ok · gate_path_roundtrip_witness 1 ok · integration 47 ok ·
priority_semantics 10 ok · priority_validation_test 5 ok · registry_ontology_test 3 ok ·
routing_dispatch 6 ok · routing_integration 21 ok
```

Failures OUTSIDE this lane's surface (sibling CL-E/CL-F spawn/timeout tests over
`server_handle.rs`/`child_process.rs`, files this lane is excluded from and which are
mid-edit — `child_eviction_test` does not compile: E0308/E0425
`bounded_request_via_pool`):

- `diagnostic_relay_probe::run_stdio_relay_works_end_to_end` (spawn relay e2e, panic at
  diagnostic_relay_probe.rs:83)
- `request_arbitration::dead_child_folds_to_none_first_request_slow_then_fast` (43s
  timeout path)

Both exercise child-process spawn/timeout, not priority mapping; the configs they use
declare "full"/"diagnostics-only", whose tier behavior is byte-identical before and
after this fix. Classified: sibling in-flight surface, not a regression of this diff.

## Ownership crossings (disclosed, minimal, SLA-legal)

1. registry.rs from_config mapping (5-line body change + doc comment). The contract
   named from_config as the fix surface while the lane exclusion listed "registry
   files"; parity (item 2) was mechanically impossible without it. 5-line mapping
   change only — spawn/registry plumbing untouched.
2. priority_semantics.rs test 10 flip — the pinned-hazard test must flip when the
   hazard is fixed; new pin asserts load-refusal + parity.
3. andon_prefixes.rs fixture "primary"→"full" — "primary" is outside the contracted
   accepted set; identical tier semantics; disclosed in both lanes' receipts per law.
