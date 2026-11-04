# lm-1 — IMPLEMENTATION lane LM-1, Wave 2 (G2–G6)

Date: 2026-11-03. Repo: /Users/sac/lsp-max (canonical checkout; no git run, lane law).
Order executed: G2 → G3 → G6 → G4 → G5 (G6 pulled ahead of G4/G5 because its
scaffold-copy drift was witnessed pre-landing).

## Landed diff (owned files only)

- `CHANGELOG.md` — [26.11.3] gains F4/CL-F/CL-H/pin-law bullets; the stale
  "In progress (F4)" line flipped to landed state.
- `docs/jira/v26.11.3/receipts/xg6-lsp-types-max-pin.md` — pin law: lsp-max
  workspace stays at `lsp-types-max = "26.6.24"` (crates.io max, re-read this
  session via real `GET crates.io/api/v1/crates/lsp-types-max` → max_version
  26.6.24, versions [26.6.24, 26.6.8]); the 5 pin lines across 4 manifests
  inventoried; adjacent stale externals (affidavit 26.6.22, wasm4pm 26.6.11,
  wasm4pm-compat 26.6.29, wasm4pm-cognition 26.6.10, clap-noun-verb 26.6.2)
  recorded so "bumped all crates" stops overreaching. NOT bumped, per contract.
- `tests/test_gate_path_byte_equal_court.rs` — new 4-test court over
  include_str!ed authoritative (`src/primitives/gate_path.rs`) + 2 scaffold
  copies. Receipt: `xg6-gate-path-court.md`. **Both scaffold copies had ALREADY
  drifted** (no `LSP_MAX_AGENT_ID` branch) and were repaired to the
  authoritative formula before the court could honestly pass; positive control
  refuses an all-copies-drop-the-branch regression.
- `crates/lsp-max-compositor/compositor.ttl` — `lspmax:command` property +
  command on `claude-code-config-lsp` (real sibling binary name, verified in
  `/Users/sac/claude-code-config-lsp/Cargo.toml` [[bin]]); absent for the other
  5 servers, per contract.
- `crates/lsp-max-compositor/queries/registry_servers.rq` — `?command` via
  OPTIONAL, per PG-5's corrected query.
- `crates/lsp-max-compositor/registry_init.tera` + regenerated
  `src/registry_init.rs` — commands field rendered through the real ggen
  pipeline (scratch ggen sync run, transplanted byte-identically).
- `crates/lsp-max-compositor/src/registry.rs` — `ChildServer.commands`
  (`Vec<String>`), `ExtensionRouter::servers_for_command` (dedup by id),
  `ChildServer::spawn_command()` typed seam returning `SpawnRefusal` for
  command-less servers; `from_config` fills `commands: Vec::new()` (contract
  reading: the ontology manifest is the command authority; the lsp-max.toml
  `ServerEntry.command` field exists but was NOT wired — disclosed, CL-G owns
  config.rs).
- `crates/lsp-max-compositor/tests/registry_ontology_test.rs` — 3 new courts
  (spawn-capable registry, command-less typed refusal, generated-surface
  commands rendering); 6/6 green.
- `src/client/builder.rs` — `PendingId` u64|String enum for inbound ids;
  canned server→client responses via tx (`workspace/configuration` →
  `vec![Null; items.len()]`, default `Null`) with warn-on-closed-write-path;
  `NotificationCounters` (progress/not_routable/malformed, AtomicU64, Clone,
  Arc-shared into read_loop); malformed-param warn paths increment malformed;
  unknown notifications and undeliverable canned answers increment
  not_routable; write_loop serialize failure → loud `panic!` with context
  (PG-5 pick disclosed: serde_json::Value serialization is infallible by
  construction, so the arm is unreachable — warn would launder a broken
  invariant; panic carries the context).

## SLA-6 minimal unblocks (disclosed)

Adding the `commands` field to `ChildServer` breaks every struct literal;
mechanical `commands: Vec::new()` inserted at: `src/fanout.rs` (5),
`tests/priority_validation_test.rs` (1), `tests/routing_dispatch.rs` (2),
`tests/integration.rs` (4), `tests/routing_integration.rs` (3),
`tests/request_arbitration.rs` (5 — CL-E-owned file, disclosed per SLA-6),
`crates/lsp-max-cli/src/bin/lsp-max-mcp.rs` (2), plus 11 in the regenerated
`registry_init.rs`. No behavior changed at any site.

## Compile-freeze disclosure (coordinator escalation)

My builder.rs edits introduced E0308 (progress match arms u64 vs ()) and E0382
(moved `tx`) that froze the workspace compile for a window; both were fixed
(fork-armed the E0308 arm, `tx.clone()`) and `cargo check -p lsp-max --lib`
cleared before the escalation was fully processed. Also disclosed: CL-E's
in-flight alive-channel plumbing arrived in builder.rs (my file) mid-edit and
was reconciled as owner (kept the plumbing, fixed the move error); a mid-save
race on server_handle.rs produced a transient E0061 that cleared on recheck.

## Engine divergence — ggen 26.10.11 SPARQL OPTIONAL cross-join (KD row)

`OPTIONAL { ?server lspmax:command ?command }` in registry_servers.rq does NOT
left-join per solution: ggen 26.10.11 cross-joins the OPTIONAL binding onto
every row (witnessed: all 11 generated registrations received
claude-code-config-lsp's command). Workaround landed: ttl convention
(`lspmax:command` value = the server's own binary name = its rdfs:label) +
template guard `row.command == row.id`. Generated output is correct under the
convention; a future ggen fix lets the guard simplify to `row.command`.
Until then, adding a server whose command != its id requires template work —
named limitation, not silent.

## Gates (commands + exits, this session)

- `cargo test -p lsp-max-compositor --test registry_ontology_test` → 6 passed;
  0 failed, exit 0.
- `cargo test -p lsp-max-compositor --lib` → 52 passed; 0 failed, exit 0.
- `cargo test -p lsp-max-compositor --test integration` → 47 passed, exit 0;
  `--test e2e` → 2 passed; `--test routing_dispatch` → 6 passed.
- `cargo test --test test_gate_path_byte_equal_court` → 4 passed; 0 failed, exit 0.
- `cargo check --workspace` → exit 0.
- Full `cargo test -p lsp-max-compositor --no-fail-fast` (pre-regeneration
  full run, all 20 suites): my surfaces green; **recorded, not fixed** (CL-E's
  owned files): child_eviction_test 2 failed, crash_isolation 2 failed,
  diagnostic_relay_probe 1 failed, server_handle_failure_test 1 failed.
  request_arbitration passed 9/9 (contract predicted it failing — it isn't).
- A second full-suite background rerun was killed at the harness time limit
  mid-`child_eviction`; the targeted reruns above cover every suite my diff
  can affect. The four failing suites are CL-E-owned files my diff does not
  touch.

## Standing

- ADMITTED: G2, G3, G6, G4 courts+gates, G5 builder machinery — all gates
  executed with exits above.
- PARTIAL: compositor suite as a whole (CL-E-owned failures recorded).
- OPEN: `from_config` does not surface `ServerEntry.command` (CL-G surface);
  court widening to `crates/lsp-max-compositor/src/gate_file.rs`; ggen
  OPTIONAL cross-join upstream fix (guard simplification).
