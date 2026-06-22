# lsp-max-scaffold — Agent Constitution

Read this file before opening any source file. It is the decision surface that
tells you which laws apply, which types carry meaning, and which anti-patterns
are structurally blocked.

---

## Quick Navigation for Coding Agents

| Task | File | Key type |
|------|------|----------|
| Detect and replay-verify a finding | `src/verifiable.rs` | `VerifiableEngine`, `verify_receipt` |
| Write a pure analyzer (detection rules) | `src/analyzer.rs` | `ReplayableAnalyzer`, `DefaultAnalyzer` |
| Record session events (OCEL 2.0) | `src/session_conformance.rs` | `SessionLog`, `SessionEvent` |
| Replay a session against Declare model | `src/session_conformance.rs` | `replay_session`, `ReplayResult` |
| Check Oracle class violations | `src/session_conformance.rs` | `OracleClass`, `OracleClassHit` |
| Check law-axis admission state | `src/law.rs` | `ScaffoldConformanceVector` |
| Emit proof-carrying LSP diagnostics | `src/server.rs` | `ScaffoldServer`, `diagnostics_for` |
| Scaffold diagnostic codes | `src/diagnostics.rs` | `ScaffoldDiagnostic`, `codes::*` |
| Read/write the ANDON gate | `src/nouns/gate.rs` | `GateService` |
| Scan a file and verify its receipt chain | `src/nouns/verify.rs` | `VerifyService` |
| Replay a persisted session log | `src/nouns/session.rs` | `SessionService` |
| Promote an axis to ADMITTED | `src/nouns/admit.rs` | `AdmitService` |

---

## What this is NOT

This is **not** a hexagonal-architecture scaffold:

| Hexagonal concept | lsp-max replacement |
|---|---|
| Domain entities (Item, Id<T>) | Law axes (Protocol, Receipt, Gate, Ontology) |
| Ports/adapters (inbound/outbound) | LSP surface (read-only) + max/* methods |
| Application services | Noun/verb service tier |
| Infrastructure (SQL, HTTP) | Receipt chain + ANDON gate |
| Error hierarchy (CoreError) | ConformanceVector (tri-state) |
| SemVer | CalVer (YY.M.D) |
| Stable toolchain | `nightly-2026-04-15` |

---

## Headline: Replay-Verifiable Diagnostics (RVD)

This scaffold's first innovation: every diagnostic carries a **proof**. A witness
(minimal reproducing input) plus a receipt (BLAKE3 digests in a hash chain) let
any verifier replay the finding and confirm it — without trusting the emitter.
Forged or tampered diagnostics fail replay and are `REFUSED`.

```
diagnostic ─┬─► witness   { doc_span, snippet_hex }
             └─► receipt   { input_digest, output_digest, prev, chain_digest }
                              └─► linked into a BLAKE3 hash chain
```

See `src/verifiable.rs`, `src/analyzer.rs`, `tests/verifiable.rs`, and
`docs/RVD.md`.

---

## Session Extension: Process-Mined Session Conformance (PMSC)

RVD proves individual findings are honest. PMSC closes the gap at the session
level: an adversarial *process* can emit individually-honest receipts while
violating causal, temporal, and epistemic laws across the session trace.

**What PMSC adds** (van der Aalst Declare + Oracle taxonomy):

| Gap RVD cannot see | Oracle class |
|---|---|
| Receipt exists before any analysis ran | A10 CausalViolation |
| Finding produced after chain was reported broken | A9 TemporalAnomaly |
| `UNKNOWN` axis collapsed without `ReceiptVerified` evidence | A11 UnknownCollapse |
| `ChainVerified(intact)` follows a refused receipt | A8 AuditTampering |
| Gate blocked in a non-terminating loop | A12 CyclicDependency |

Every session event is an **OCEL 2.0** object-centric entry bound simultaneously
to six object types: `Document`, `Finding`, `Receipt`, `Gate`, `AxisState`,
`RuleSet`. A Declare constraint model encodes process laws. Token replay computes
the van der Aalst fitness metric.

See `src/session_conformance.rs`, `tests/session_conformance.rs`, and the PMSC
section in `docs/RVD.md`.

---

## The Five-Layer Model

```
Layer 1 — Actuation Grammar     src/nouns/                 clap-noun-verb CLI
Layer 2 — Local LSP State       src/server.rs              LanguageServer impl
Layer 3 — Law-State Runtime     src/law.rs                 ConformanceVector, AxisState
          Proof surface         src/verifiable.rs           receipts, witnesses, hash chain
          Session conformance   src/session_conformance.rs  OCEL 2.0 log, Declare, Oracle A8–A12
          Analyzer              src/analyzer.rs             pure, replayable detection
          Diagnostics           src/diagnostics.rs          ScaffoldDiagnostic, codes
Layers 4/5 — (extend here)     lsp-max-runtime             AutonomicMesh, mesh routing
```

---

## Bounded Status Vocabulary

**Allowed:** `ADMITTED`, `CANDIDATE`, `BLOCKED`, `REFUSED`, `UNKNOWN`,
`PARTIAL`, `OPEN`

**Forbidden (victory language):** `done`, `solved`, `guaranteed`, `complete`,
`supported`, `fully`, `all clean`

Violations trigger `ANTI-LLM-CHEAT-*` diagnostics from the canary server.

---

## Law #1 — UNKNOWN Must Not Collapse

`ScaffoldConformanceVector` has three disjoint sets: `admitted`, `refused`,
`unknown`. An axis begins in `unknown`. It may only move to `admitted` via
`admit_axis()` — and only after all three receipt-chain preconditions are met:

1. A receipt exists in `receipts/<method>.json` with a valid BLAKE3 digest
2. A transcript exists in `transcripts/<method>.jsonl`
3. A negative-control exists in `fixtures/negative_controls/<method>.rs`

Calling `admit_axis()` before these artifacts exist is a law violation. The
negative control in `fixtures/negative_controls/unknown_collapse.rs` demonstrates
the violation pattern.

PMSC makes this law directly detectable at the session level: an
`AxisTransitioned(Unknown→Admitted)` event without a prior `ReceiptVerified`
event triggers **Oracle class A11**.

---

## Law #2 — LSP Surface is Read-Only

`ScaffoldServer` (Layer 2) emits diagnostics, hovers, and code actions. It
never writes files, never runs processes, never mutates workspace state. The
ANDON gate check in `did_open`/`did_change`/`did_save` enforces this boundary.

---

## Law #3 — ANDON Gate

Before any shell action, the gate file must be clear. The `gate` noun reads the
workspace-specific gate file (`/tmp/lsp-max-gate-<hash>`) and returns exit 1
if blocked. The `server.rs` gate check runs on every document event.

Gate file path formula (must match `lsp-max-compositor/src/gate_file.rs`):
```
XDG_RUNTIME_DIR/lsp-max-gate-<fnv1a(workspace_path)>
```

A cyclic sequence of `GateChecked(blocked=true)` events (≥ 5 without resolution)
triggers **Oracle class A12** during PMSC replay.

---

## Law #4 — Receipt Chains

Every method claim requires a receipt. A receipt has:
- `"boundary": "-----BEGIN RECEIPT-----"`
- `"checkpoint": "-----END RECEIPT-----"`
- `"digest_algorithm": "BLAKE3"`
- `"digest": "<hex of transcript>"` (not a placeholder)
- `"status": "CANDIDATE"` (promoted to `"ADMITTED"` after `admit promote`)

```sh
lsp-max-scaffold admit receipt --method <method>   # generate skeleton
lsp-max-scaffold admit check   --method <method>   # verify preconditions
lsp-max-scaffold admit promote --method <method>   # promote to ADMITTED
```

---

## Law #5 — CalVer

Version is `version.workspace = true` — inherits the workspace CalVer
(`YY.M.D`). Never introduce a SemVer bump or a `version = "0.x"` in any crate.

---

## Law #6 — Diagnostics Carry Proofs (RVD)

A diagnostic without a witness and a verifying receipt is `UNKNOWN`, never
trusted. Analyzers (`analyzer::ReplayableAnalyzer`) must be **pure** — no clock,
no RNG, no I/O — because replay on the witness must reproduce the finding
exactly. The verifier (`verifiable::verify_receipt`) replays the witness, never
the emitter's claim.

```sh
lsp-max-scaffold verify scan  --file <path>          # build + replay-verify a chain
lsp-max-scaffold verify chain --file <receipts.json> # check hash-chain linkage
```

Over the LSP wire, every `Diagnostic` carries its proof in `data`:
`{ "receipt": {...}, "witness": {...} }` — any client can re-verify.

---

## Law #7 — Sessions Must Conform (PMSC)

Per-finding receipts prove individual honesty; session conformance proves
process honesty. A session log that passes RVD receipt checks can still violate
causal and temporal laws. PMSC replay catches these.

### Recording events

Append to a `SessionLog` as the session progresses:

```rust
use lsp_max_scaffold::session_conformance::{EventActivity, EventObjects, SessionLog};

let mut log = SessionLog::new();
log.append(
    EventActivity::AnalysisRun { source_digest: digest },
    EventObjects { document: Some(uri), ruleset: Some(version), ..Default::default() },
);
```

### Replaying a session

```rust
use lsp_max_scaffold::session_conformance::replay_session;

let result = replay_session(&log);
// result.fitness   ∈ [0, 1]
// result.status    ∈ bounded vocabulary
// result.violations  — Declare constraint failures
// result.oracle_hits — A8–A12 classification
```

### CLI

```sh
# Deserialize a persisted SessionLog JSON and report fitness + violations.
lsp-max-scaffold session replay --file session.json
```

### Oracle Class Quick Reference

| Class | Trigger (first occurrence) |
|-------|---------------------------|
| A8 AuditTampering | `ChainVerified(intact)` after any `ReceiptVerified(refused)` |
| A9 TemporalAnomaly | `FindingProduced` after `ChainVerified(intact=false)` |
| A10 CausalViolation | `ReceiptProduced` without prior `AnalysisRun` |
| A11 UnknownCollapse | `AxisTransitioned(Unknown→Admitted\|Refused)` without prior `ReceiptVerified` |
| A12 CyclicDependency | `GateChecked(blocked)` repeated ≥ 5 times without reset |

The `from`/`to` values in `AxisTransitioned` must use the bounded vocabulary
strings: `"Unknown"`, `"Admitted"`, `"Refused"`.

---

## Noun/Verb Pattern

Each file in `src/nouns/` is a noun with three tiers:

```rust
// 1. Domain tier — serialisable value types
#[derive(Debug, Serialize)]
pub struct MyResult { pub status: &'static str }

// 2. Service tier — pure logic, no I/O side-effects in the core
pub struct MyService;
impl MyService { pub fn new() -> Self { Self } }
impl Default for MyService { fn default() -> Self { Self::new() } }

// 3. Verb tier — #[verb] entry points
#[verb("do-thing")]
pub fn do_thing() -> clap_noun_verb::Result<MyResult> {
    Ok(MyService::new().do_thing())
}
```

Nouns in this scaffold: `admit`, `gate`, `serve`, `session`, `verify`.

---

## Key Types for Agents

When writing code that touches this scaffold, reach for these types directly:

```
verifiable::VerifiableEngine    — builds proof-carrying diagnostics from source text
verifiable::verify_receipt      — replays a witness; returns AxisState
verifiable::verify_chain        — checks hash-chain linkage; returns ChainVerdict
verifiable::Receipt             — BLAKE3 digest tuple (input, output, prev, chain)
verifiable::Witness             — { doc_span, snippet_hex } (hex-encoded, no forbidden tokens)

analyzer::ReplayableAnalyzer    — trait: pure (version, ruleset, source) → Vec<RawFinding>
analyzer::DefaultAnalyzer       — production impl (fork-ref + victory-lang rules)
analyzer::Rule                  — { code, patterns: Vec<String>, message_prefix }

session_conformance::SessionLog       — OCEL 2.0 log; append() + digest()
session_conformance::SessionEvent     — { seq: u64, activity, objects }
session_conformance::EventActivity    — tagged union of all session event kinds
session_conformance::EventObjects     — multi-type object bindings per event
session_conformance::replay_session   — → ReplayResult { fitness, violations, oracle_hits, status }
session_conformance::OracleClass      — A8..A12 enum
session_conformance::DeclareConstraint — Response | Precedence | Absence | NotCoexistence

law::ScaffoldConformanceVector  — { admitted, refused, unknown } tri-state
law::ScaffoldAxis               — Protocol | Receipt | Gate | Ontology | Custom(String)
law::AxisState                  — Admitted | Refused | Unknown

diagnostics::ScaffoldDiagnostic — { code, status, message, repair }
diagnostics::codes::*           — SCAFFOLD-RECEIPT-001, SCAFFOLD-GATE-001, SCAFFOLD-AXIS-001
```

---

## CI

The workspace CI (`ci.yml`) covers this crate. It uses `nightly-2026-04-15`
and checks out all three sibling repos before running `cargo fmt -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`, and
`cargo test --workspace`.

The test suites that directly cover the new modules:

| File | What it tests |
|------|---------------|
| `tests/verifiable.rs` | RVD: four tamper vectors (V1–V4), chain linkage, determinism |
| `tests/conformance.rs` | ConformanceVector tri-state invariants |
| `tests/session_conformance.rs` | PMSC: each Oracle class, Declare constraints, round-trip |

---

## Anti-Patterns (Enforced)

1. **Upstream-fork references** — the `tower` stem joined with `-lsp` / `_lsp` in
   code or docs → ANTI-LLM-CHEAT-* (blocked by canary). Use `lsp-max`.

2. **UNKNOWN coerced to ADMITTED without receipts** → SCAFFOLD-AXIS-001.
   PMSC additionally catches this at session level as Oracle A11.

3. **Victory language in messages/statuses** → ANTI-LLM-CHEAT-VICTORY-*.
   Also blocked by the canary's `RVD-VICTORY-001` rule in production.

4. **Exhaustive struct literals for LSP types** → breaks when `lsp-types-max`
   adds fields. Always use `..Default::default()` for trailing fields.

5. **`cargo test` output as admission evidence** → ANTI-LLM-CHEAT-RECEIPT-INVALID.
   Test stdout is not a receipt. Use `verify scan` for receipt-quality evidence.

6. **Impure analyzers** — clock, RNG, or I/O in `analyze()` breaks replay.
   The verifier must reproduce the finding on the witness alone. Purity is
   non-negotiable: see `analyzer::ReplayableAnalyzer`.

7. **Orphan receipts** — a `ReceiptProduced` session event without a prior
   `AnalysisRun` → Oracle A10. Session logs must preserve causal order.

8. **Post-mortem findings** — a `FindingProduced` event after
   `ChainVerified(intact=false)` → Oracle A9. A broken chain is not a valid
   continuation surface.

9. **Fake session digests** — calling `SessionLog::digest()` and recording it
   as evidence without actually replaying the log through `replay_session` is
   the PMSC analogue of a fake receipt. The digest proves content integrity;
   `replay_session` proves process conformance. Both are required.
