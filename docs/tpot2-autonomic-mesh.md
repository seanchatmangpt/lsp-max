# TPOT2 Autonomic Pipeline Mesh — Layer 5 Design (CANDIDATE)

**Status: CANDIDATE / OPEN.** This is a direction-setting design document, not an
admission. It describes how a TPOT2 breed-pipeline optimizer would be wired into
a self-regulating layer-5 loop over the ANDON gate, the receipt chain, and the
`ConformanceVector`. Nothing here is claimed as `SUPPORTED_WITH_TRANSCRIPT`.
Every proposed component carries a bounded status in
[§6](#6-component-status-table). Components present on this branch are cited by
path and symbol and verified; everything else is marked `CANDIDATE` or `OPEN` and
is **not** asserted to work.

CalVer: workspace `26.6.x` (YY.M.D), not SemVer.

---

## 0. Branch-state precondition (read this first)

This document is written on the worktree branch whose HEAD is the merge of
PR #6 (`118c2f3`). On **this branch**, the TPOT2 optimizer itself is **not
present**: there is no `src/pipeline/`, no `lsp-max-protocol/src/pipeline.rs`, no
`crates/lsp-max-cli/src/nouns/pipeline.rs`, no `tests/test_tpot2_properties.rs`,
and no `scripts/pipeline-receipt.sh` in the tree (verified by `git ls-files`).

Consequently the TPOT2 surface is treated here as a **not-yet-present dependency
of the mesh**, and every TPOT2-specific row in
[§6](#6-component-status-table) is `CANDIDATE` or `OPEN` — never `ADMITTED` —
regardless of whether a sibling branch carries an implementation. The named
TPOT2 symbols below (`PipelineSearchResult`, `PipelineBoundedStatus`,
`diagnostics_for_search()`, `TPOT2_EMPTY_POOL` / `TPOT2_NONCONVERGENCE` /
`TPOT2_OCEL_MISSING`) describe the *intended optimizer shape* this loop would
consume; on this branch they are design targets, not in-tree code, and are
labelled as such everywhere they appear.

What **is** present and verified on this branch — the law-state machinery the
mesh actuates into — is cited directly: the autonomic mesh (`AutonomicMesh`), the
`Hook`/`MeshAction` surfaces, the `ConformanceVector`, the receipt types, and the
ANDON gate service. Those are the load-bearing anchors; the design is grounded in
them.

---

## 1. The five-layer model and where this sits

The workspace's five-layer model (CLAUDE.md, AGENTS.md "Final Prime"):

```text
(1) actuation grammar          clap-noun-verb CLI               crates/lsp-max-cli
(2) local LSP state surface    diagnostics / hovers / intents    src/, lsp-max-protocol
(3) law-state runtime          typestate, conformance, receipts  lsp-max-runtime
(4) knowledge hooks            Hook trait, hook graph            lsp-max-runtime/src/mesh*
(5) autonomic LSP mesh         sense→decide→actuate→witness      <-- THIS DOCUMENT
```

Layer 5 is the closed loop that makes layers 1–4 *regulate each other* without a
human in the inner loop. The load-bearing claim of this document is narrow: the
wiring between a TPOT2 outcome, the ANDON gate signal, the `ConformanceVector`
`Domain` axis, and a receipt obligation can be expressed in terms of mesh symbols
that already exist on this branch — but the TPOT2 producer is not yet here, the
loop that ties everything together is not built, and several hops have honest
gaps.

---

## 2. Problem framing

### Why a self-regulating loop matters

A TPOT2 optimizer produces a `PipelineSearchResult` (design target) carrying a
bounded status and a best fitness. Reporting that outcome — through a
`clap-noun-verb` verb or a read-only `max/pipeline*` method — is the end of the
line: a human or an outer agent must read the status word and decide what to do.

A self-regulating loop closes that gap. When a search ends `REFUSED`
(empty breed pool) or fails to converge (`PARTIAL` / `UNKNOWN` below the
admission threshold), the *system itself* registers the condition where the rest
of the workspace already looks: the ANDON gate file and the `ConformanceVector`.
That is what "autonomic" buys — the optimizer's failure to find an admissible
pipeline becomes a first-class law-state signal, not a log line an agent might
miss.

### What "autonomic" means here

Autonomic here is the four-beat loop **sense → decide → actuate → witness**,
mapped onto symbols. Anchors marked *(present)* are verified in this tree;
*(target)* anchors are the not-yet-present TPOT2 dependency (see §0):

| Beat | Meaning | Anchor symbol |
|---|---|---|
| sense | Read a search outcome and its bounded status | `PipelineSearchResult` *(target)* |
| decide | Map the outcome to law signals, three-state | `diagnostics_for_search()` *(target)* |
| actuate | Project signals into gate state and a conformance vector | `GateService` (`crates/lsp-max-cli/src/nouns/gate.rs`) *(present)*; `build_conformance_vector()` (`lsp-max-runtime/src/mesh.rs`) *(present)* |
| witness | Bind each decision to a receipt artifact | `Receipt` (`lsp-max-protocol/src/core.rs`) *(present)*; `CryptographicReceipt` (`lsp-max-runtime/src/control_plane/receipts.rs`) *(present)* |

Two constraints frame the whole design:

- **The editor/agent is a client, not the owner.** Agents, CI, and release gates
  consume the loop's signals; the loop does not assume an editor is attached.
- **The surface is read-only (AGENTS.md §4).** The mesh emits diagnostics,
  intents, gate state, and receipts. It never mutates workspace files. Any future
  file mutation must route through
  `CodeAction → clap-noun-verb admission → … → MutationGate → Receipt`. Searching
  for a pipeline, deciding it is non-convergent, and recording that decision are
  all observation/attestation acts, not mutation.

### What this is not

This is not a claim that the loop runs, nor that the TPOT2 producer is present on
this branch. It is a claim about *how* the loop would be wired, expressed against
real mesh types so the design can be checked against the code rather than against
intentions.

---

## 3. Signal flow

### The four-beat path

```text
        sense                  decide                    actuate                    witness
  ┌───────────────┐      ┌──────────────────┐     ┌────────────────────┐     ┌──────────────────┐
  │ search outcome │      │ diagnostics_for_  │     │ gate signal +      │     │ receipt artifact │
  │ (PipelineSearch│─────▶│   search(status,  │────▶│ ConformanceVector  │────▶│ (boundary +      │
  │  Result,target)│      │   best_fitness,   │     │  Domain axis       │     │  checkpoint +    │
  │                │      │   threshold)      │     │ (build_conformance │     │  digest)         │
  │                │      │  →Vec<MaxDiagnostic>     │  _vector, present)  │     │                  │
  └───────────────┘      └──────────────────┘     └────────────────────┘     └──────────────────┘
        status                 TPOT2-* codes              ANDON / axis                proof measure
   ADMITTED|PARTIAL|       EMPTY-POOL (Error)         REFUSED→block (CANDIDATE)     binds the decision
   UNKNOWN|REFUSED         NONCONVERGENCE (Warn/Info)  Domain axis refused/unknown   (test stdout ≠ receipt)
                          OCEL-MISSING (Info)
```

### Outcome → signal mapping (the intended `diagnostics_for_search`)

The decide hop is designed as a pure function:
`diagnostics_for_search(status, best_fitness, admission_threshold) -> Vec<MaxDiagnostic>`
(target). It would assign each emitted diagnostic `law_axis = LawAxis::Domain`
(`lsp-max-protocol/src/conformance.rs`, verified present — `LawAxis::Domain`
and `LawAxisId::DOMAIN` exist). The intended mapping:

| `PipelineSearchResult.status` | Emitted `TPOT2-*` (target) | Severity | Proposed gate effect | Proposed `ConformanceVector` effect (`Domain`) |
|---|---|---|---|---|
| `REFUSED` (empty pool) | `TPOT2_EMPTY_POOL` | `ERROR` | CANDIDATE: set ANDON | refused (Error → refused) |
| `UNKNOWN` (OCEL absent) | `TPOT2_OCEL_MISSING` + `TPOT2_NONCONVERGENCE` | `INFORMATION` | OPEN: must NOT set ANDON, must NOT clear it | **must be `unknown`** (see [§4](#4-the-three-state-discipline-end-to-end) and the gap below) |
| `PARTIAL` (below threshold) | `TPOT2_NONCONVERGENCE` | `WARNING` | CANDIDATE: surface, do not block | unknown or below-bar; not admitted |
| `ADMITTED` (≥ threshold) | none | — | CANDIDATE: do not block | candidate for `Domain` admitted *only with a receipt* |

The gate on this branch is a single byte: `GateService::check()`
(`crates/lsp-max-cli/src/nouns/gate.rs`, verified) returns a `GateCheckResult`
(`andon_blocked` / `gate_file` / `compositor_active`) by reading the workspace
gate file (`gate_file_path()`, FNV-1a of cwd, matching
`crates/lsp-max-compositor/src/gate_file.rs`). The `check` verb exits 1 when
`andon_blocked`. Per AGENTS.md "Λ_CD Predicate", the gate blocks on Error-severity
diagnostics whose `law_id` is in the governed set `A` (`WASM4PM-*`, `ANTI-LLM-*`,
`GGEN-*`). For the gate to react to TPOT2, `TPOT2-*` codes would have to be
admitted into a governed prefix set. That admission is **OPEN** — a policy
decision plus a wiring change in compositor prefix routing
(`crates/lsp-max-compositor/src/merge.rs`, per AGENTS.md L7 Speciation, itself
PARTIAL), not something this document asserts.

> Note: the richer agent-context gate output (`check_agent_context`,
> `AgentContextResult`, `GoverningAxes`) described in AGENTS.md RFC-1 is **not on
> this branch** — the in-tree `gate.rs` exposes only `check()` /
> `GateCheckResult`. The loop here targets the byte-level signal that exists.

### Honest gap at the actuate hop (OPEN)

`build_conformance_vector()` (`lsp-max-runtime/src/mesh.rs`, verified) currently
classifies each diagnostic's axis as **refused iff any diagnostic on that axis
has `ERROR` severity, otherwise admitted**, and synthesizes `unknown` only for
`LawAxis::all_named()` axes that *no* diagnostic witnessed (the function iterates
`all_named()` and adds the un-witnessed axes to `unknown`). This has a direct
consequence for the intended TPOT2 wiring:

- The `UNKNOWN` path would emit `TPOT2_OCEL_MISSING` / `TPOT2_NONCONVERGENCE` at
  `INFORMATION` severity. Fed into `build_conformance_vector()` as-is, the
  `Domain` axis would be witnessed-but-not-Error, and would therefore land in
  **`admitted`** — collapsing `UNKNOWN` into `ADMITTED`.

That is precisely the collapse AGENTS.md §"Common Anti-Patterns" #5 forbids. So
the loop **cannot** hand the `UNKNOWN`-path diagnostics to
`build_conformance_vector()` unchanged. A correct layer-5 wiring needs an
explicit three-state lift (see [§4](#4-the-three-state-discipline-end-to-end))
that routes the `UNKNOWN` outcome to `ConformanceVector::set_unknown(LawAxisId::DOMAIN)`
rather than letting severity alone decide. Until that lift exists, the actuate
hop for the `UNKNOWN` outcome is **OPEN**, and the design must not pretend
otherwise.

### Where the loop would live

`MeshAction` (`lsp-max-runtime/src/mesh_types.rs`, verified) already enumerates
`AddDiagnostic`, `EmitReceipt`, and `ExecuteBoundedAction`; the `Hook` trait
(`trigger(&self, event: &HookEvent) -> Vec<MeshAction>`, verified) is the
existing extension point, and `MaxMethod::AutonomicLoop` (`max/autonomicLoop`,
verified) already appears in the method enum. A TPOT2 layer-5 hook would be a
`Hook` implementation that, on a search-outcome event, returns `AddDiagnostic`
(the `TPOT2-*` diagnostics) and `EmitReceipt` (the witness). It is **CANDIDATE**:
the mesh surfaces exist; no such hook is registered, and `HookEvent`
(`lsp-max-protocol/src/hooks.rs`, verified) has no search-outcome variant today
(its variants are `StateTransition`, `DiagnosticEmitted`, `DiagnosticCleared`,
`ReceiptEmitted`, `PolicyStateChanged`, `BoundedActionExecuted`,
`InstanceReset`).

---

## 4. The three-state discipline end to end

The non-negotiable law (AGENTS.md §8, CLAUDE.md anti-patterns #5): **`UNKNOWN` is
never collapsed into `ADMITTED` or `REFUSED` at any hop.** The loop has four
hops, and the third state must survive each one.

### Hop-by-hop carriage of `UNKNOWN`

1. **sense.** The intended `PipelineBoundedStatus` (target) carries a distinct
   `Unknown` variant alongside `Admitted` / `Partial` / `Refused` / `Blocked`.
   `Unknown` enters the loop as itself. (Negative-control and bounded-variant
   property coverage is a TPOT2-side obligation, CANDIDATE on this branch.)

2. **decide.** `diagnostics_for_search()` (target) keeps `UNKNOWN` distinct: on
   the `UNKNOWN` arm it emits `TPOT2_OCEL_MISSING` + `TPOT2_NONCONVERGENCE` at
   `INFORMATION` severity and explicitly does **not** emit the Error-severity
   `TPOT2_EMPTY_POOL`. At the decide hop, `UNKNOWN` is carried as "informational,
   not refused, not admitted."

3. **actuate.** This is the fragile hop (see
   [§3](#honest-gap-at-the-actuate-hop-open)). The three-state lift the loop
   needs, expressed against the **verified** `ConformanceVector` API
   (`lsp-max-protocol/src/conformance.rs`):
   - `REFUSED` outcome → `ConformanceVector::set_refused(LawAxisId::DOMAIN)` and
     (CANDIDATE) raise ANDON.
   - `UNKNOWN` outcome → `ConformanceVector::set_unknown(LawAxisId::DOMAIN)` and
     **leave ANDON untouched** — an absent OCEL source or an untraced
     precondition is a gap, not a violation. `ConformanceVector` supports this
     directly: the disjoint bitmask setters (`set_admitted` / `set_refused` /
     `set_unknown`) enforce mutual exclusion via `assert_bitmask_invariants()`,
     and `admits_release()` blocks on `unknown` when `strict_mode` is true
     (verified by the in-crate tests `admits_release_strict_mode_blocks_unknown`
     and `unknown_blocks_strict_only`). The three-state machinery is real; the
     missing piece is the *routing* that sends the `UNKNOWN` outcome to
     `set_unknown` instead of letting `build_conformance_vector()`'s severity
     heuristic file it as `admitted`.
   - `ADMITTED` outcome → eligible for `set_admitted(LawAxisId::DOMAIN)` **only
     with a receipt** ([§5](#5-receipt-obligations)).

4. **witness.** A receipt records which polarity was actuated. The receipt for an
   `UNKNOWN` outcome must record `UNKNOWN` — it must not be a "no receipt =
   implicitly fine" silence (which would read as `ADMITTED`) nor a refusal
   receipt.

### The standing rule

`Unknown` is structurally distinct in every type the loop touches that exists on
this branch: `AdmissionDecision::Unknown` (`lsp-max-protocol/src/hooks.rs`,
verified — and `From<bool>` deliberately yields only `Admitted`/`Refused`, never
`Unknown`, so a boolean can never *manufacture* the third state — asserted by the
in-crate test `admission_decision_into_bool`), `ConformanceVector.unknown` /
`unknown_bits`, and `Repairability::Unknown`
(`lsp-max-protocol/src/diagnostics.rs`, verified). The loop's contribution is to
not erase that distinction at the actuate hop. The runnable contract witness
`examples/conformance_vector_explained.rs` (referenced by the `ConformanceVector`
docstring) asserts the no-collapse law for the vector itself; a layer-5 analogue
(asserting the TPOT2 `UNKNOWN` outcome lands in `ConformanceVector.unknown`, not
`.admitted`) is **OPEN** and would be the first thing built to make this hop
trustworthy.

---

## 5. Receipt obligations

### The standing rule (AGENTS.md §6)

**Test stdout, log lines, and status words are not receipts.** A printed
`ADMITTED`, a `test result: ok`, or a search summary string proves nothing:
`StatusWord(ADMITTED) ⇏ Admitted`. Every autonomic decision that moves a
`ConformanceVector` axis or touches the gate must bind a receipt artifact — or it
is not admitted, full stop.

### What each decision must bind

A receipt is the proof measure for an actuate decision. Per AGENTS.md §6
("Receipts must bind"), the bound fields:

| Field | Source in the loop | Anchor |
|---|---|---|
| `boundary` / `checkpoint` | constant markers (`-----BEGIN/END RECEIPT-----`) | marker-receipt convention (AGENTS.md §6); emitter is a TPOT2-side target |
| `digest` + `digest_algorithm` | digest over the decision's bound inputs | content addressing as in `Receipt.hash` (`lsp-max-protocol/src/core.rs`) |
| `raw_command` | the `clap-noun-verb` invocation that produced the outcome | e.g. `lsp-max-cli pipeline search …` (target verb) |
| bounded `status` | the actuated polarity (`ADMITTED`/`REFUSED`/`UNKNOWN`/`PARTIAL`/`BLOCKED`) | bounded status set (AGENTS.md §8) |
| negative-control result | when required, an empty-pool `REFUSED` control | TPOT2-side property obligation (CANDIDATE on this branch) |

For the chained / cryptographic variant, the in-tree authority (verified) is
`CryptographicReceipt` (`lsp-max-runtime/src/control_plane/receipts.rs`): it binds
`prev_hash`, `consequence_hash`, `sequence`, and an Ed25519 `signature`, and
`verify_receipt_chain()` checks sequence progression, link integrity, payload
digest, and signature. A layer-5 loop that emits a *chain* of decisions (one per
iteration) would use `MeshAction::EmitReceipt` carrying the protocol-level
`Receipt` (`lsp-max-protocol/src/core.rs`, verified — it has `prev_receipt_hash`
for Merkle chaining and a runnable witness `examples/receipt_chain_explained.rs`),
and could graduate to `CryptographicReceipt` for signed attestation.

### The honest limitation the loop inherits

A marker receipt that digests only `breeds | fitness | ocel_path` binds the
**claim**, not a re-executed engine run. A receipt that proves a wasm4pm engine
actually produced the fitness (an output digest of a real `wasm4pm breed run`) is
**CANDIDATE**. The autonomic loop cannot manufacture stronger evidence than the
receipt it binds: if the underlying fitness is heuristic (the wasm4pm engine
absent), the receipt witnesses a heuristic decision, and the `Domain` axis it
admits is admitted *as a heuristic decision*, not as an engine result. The loop
must not launder a heuristic score into an engine claim.

---

## 6. Component status table

Each proposed mesh component, with what exists on this branch versus what is
`CANDIDATE` or `OPEN`. **No row is `SUPPORTED_WITH_TRANSCRIPT`** — this is a
design document, and that status requires a transcript + negative control +
receipt that this document does not produce. "*(present)*" marks anchors verified
in this tree; "*(target)*" marks the not-yet-present TPOT2 dependency (§0).

| Component | What it would do (layer 5) | Anchor (path / symbol) | Status |
|---|---|---|---|
| TPOT2 search outcome producer | Emit a bounded `PipelineSearchResult` to sense | `PipelineSearchResult` *(target)* | OPEN (not on this branch; §0) |
| Outcome → diagnostics decide | Map status + fitness to `TPOT2-*`, three-state | `diagnostics_for_search()` *(target)* | OPEN (not on this branch; §0) |
| Three-state `Domain`-axis lift | Route outcome to `set_admitted/refused/unknown(DOMAIN)` | `ConformanceVector::set_*` (`lsp-max-protocol/src/conformance.rs`) *(present)*; `build_conformance_vector()` (`lsp-max-runtime/src/mesh.rs`) *(present)* | OPEN (setters exist; severity heuristic would file `UNKNOWN`-path info diagnostics as `admitted`; explicit routing not built) |
| TPOT2 gate admission | Admit `TPOT2-*` Error codes into a governed prefix set so ANDON reacts | `GateService` (`crates/lsp-max-cli/src/nouns/gate.rs`) *(present)*; `crates/lsp-max-compositor/src/merge.rs` prefix routing *(present)* | OPEN (TPOT2 codes not in any governed set; per-server routing itself PARTIAL per AGENTS.md L7) |
| Layer-5 search-outcome hook | A `Hook` returning `AddDiagnostic` + `EmitReceipt` on a search event | `Hook`, `MeshAction` (`lsp-max-runtime/src/mesh_types.rs`) *(present)*; `HookEvent` (`lsp-max-protocol/src/hooks.rs`) *(present)* | CANDIDATE (mesh surfaces exist; no hook registered; `HookEvent` has no search-outcome variant) |
| Autonomic loop method | Drive sense→…→witness per iteration | `MaxMethod::AutonomicLoop` (`lsp-max-runtime/src/mesh_types.rs`) *(present)*; `AutonomicLoopStatus` (`lsp-max-protocol/src/hooks.rs`) *(present)* | CANDIDATE (method enumerated; TPOT2-specific loop body not built) |
| Marker-receipt witness | Bind each decision to a boundary/checkpoint/digest receipt | `Receipt` (`lsp-max-protocol/src/core.rs`) *(present)*; emitter/validator scripts *(target)* | CANDIDATE (protocol type present; TPOT2 emitter/validator scripts not on this branch) |
| Cryptographic receipt chain | Signed, sequence-linked attestation across iterations | `CryptographicReceipt`, `verify_receipt_chain()` (`lsp-max-runtime/src/control_plane/receipts.rs`) *(present)* | CANDIDATE (chain machinery present; not wired to TPOT2 outcomes) |
| Engine-backed evidence | Receipt over a real `wasm4pm breed run` output digest | wasm4pm engine + a TPOT2 subprocess evaluator *(target)* | CANDIDATE (heuristic-only without the engine; no engine-run receipt) |
| Breed-affinity feedback | Bias mutation sampling from receipt-witnessed history | `ConformanceDeltaEntry` (`lsp-max-runtime/src/mesh_types.rs`) *(present)*; TPOT2 catalog *(target)* | CANDIDATE (not built; §7.1) |
| Convergence hysteresis | Escalate an advisory signal after N non-convergent runs | `ConformanceDeltaEntry` (`lsp-max-runtime/src/mesh_types.rs`) *(present)* | CANDIDATE (not built; §7.2) |
| No-collapse contract witness | A test asserting TPOT2 `UNKNOWN` lands in `.unknown`, not `.admitted` | analogue of `examples/conformance_vector_explained.rs` *(present, as model)* | OPEN (does not exist; prerequisite for trusting the actuate hop) |

---

## 7. Feedback — convergence history biasing future search (CANDIDATE)

This section is entirely **CANDIDATE**. None of it is built; it sets direction.

A genuinely autonomic loop would not just record outcomes — it would let the
history of outcomes bias the next search. Two concrete, bounded proposals:

### 7.1 Breed-affinity priors

A TPOT2 catalog (target) partitions breeds into categories, and a genetic engine
draws replacement breeds uniformly at random during mutation/crossover. A
feedback loop could maintain an *affinity prior* per breed (or per category)
derived from the `ConformanceDeltaEntry` history (`lsp-max-runtime/src/mesh_types.rs`,
verified — it records `old_score` → `new_score` per instance with a timestamp)
and the receipt chain of prior `ADMITTED` pipelines. Mutation would then sample
from a non-uniform distribution weighted by affinity, rather than uniformly. This
changes how the search *samples*, not what counts as admissible — the admission
threshold and the three-state discipline are untouched.

Boundedness requirements that keep this honest:
- The prior must be derived **only from receipt-witnessed outcomes**, never from
  raw status words or test stdout. An unwitnessed `ADMITTED` must not raise any
  breed's affinity.
- Determinism must be preserved: a seed-deterministic engine PRNG makes the
  search seed-reproducible. A prior makes it *seed × prior*-deterministic; the
  prior itself must be a pure function of a receipt-chain snapshot, or replay
  breaks.

### 7.2 Convergence-history gate hysteresis

If repeated searches over the same OCEL log keep landing `PARTIAL`/`UNKNOWN`, the
loop could (CANDIDATE) escalate: after N receipt-witnessed non-convergent runs,
raise an advisory `Domain`-axis signal that an outer agent or CI can read,
**without** raising ANDON (non-convergence is not a law violation). This is
feedback on the *meta* outcome (the search keeps not converging) rather than a
single run. It stays read-only and never blocks; it only makes a standing
condition legible.

Both 7.1 and 7.2 are explicitly out of scope to *build* here. They are recorded
so the direction is on the record and can be designed against the real
`ConformanceDeltaEntry` / receipt-chain types rather than reinvented.

---

## 8. Risks and non-goals

### Non-goals (explicit)

- **The mesh never mutates workspace files.** Layer 5 is read-only by law
  (AGENTS.md §4). It emits diagnostics, code-action *intents*, gate state, and
  receipts. It does not write source, edit configs, or apply pipelines to disk.
  The "actuate" beat actuates *law state* (gate byte, conformance axis, receipt),
  not files. Any file mutation is out of scope and would have to route through
  `CodeAction → clap-noun-verb admission → PackActionIntent → PackPlan → Staging
  → MutationGate → Receipt`.
- **No victory closure.** The loop never concludes that the workspace is
  "fine"/"clean". It moves bounded statuses; the absence of a refusal is not an
  admission.
- **Not a scheduler or a daemon design.** How often the loop runs, and on what
  trigger, is out of scope. This document specifies the *signal wiring*, not an
  execution cadence.

### Risks

- **Three-state collapse at the actuate hop (the central risk).** As detailed in
  [§3](#honest-gap-at-the-actuate-hop-open) and
  [§4](#4-the-three-state-discipline-end-to-end), feeding `INFORMATION`-severity
  `UNKNOWN`-path diagnostics to `build_conformance_vector()` unchanged would file
  the `Domain` axis as `admitted`. This is the most likely way the loop could
  silently violate the no-collapse law. Mitigation: an explicit three-state lift
  routing the outcome to `ConformanceVector::set_unknown`, plus a contract-witness
  test (both OPEN).
- **Gate over-reach.** Admitting `TPOT2-*` into a governed ANDON prefix set could
  let a non-convergent search block unrelated shell actions
  (`Λ_CD^runtime`, CLAUDE.md / AGENTS.md). Mitigation: only `TPOT2_EMPTY_POOL`
  (a true `REFUSED`) is a candidate for ANDON; `NONCONVERGENCE` / `OCEL_MISSING`
  must be advisory only. This boundary is a policy decision, marked OPEN.
- **Receipt laundering.** The loop could appear to "prove" admission by binding a
  receipt that only digests a status word. Mitigation: the receipt must bind the
  `raw_command` and the decision inputs, and an engine claim requires an
  engine-run digest (CANDIDATE).
- **Subagent gate boundary.** Per AGENTS.md "Subagent Gate Propagation — Status:
  OPEN", `PreToolUse` hooks do not cross `Agent` session boundaries. A layer-5
  loop driven from a subagent does not inherit the parent's gate enforcement; the
  canonical single-syscall `lsp-max-cli gate check` (`GateService::check`,
  verified) remains the only portable check. This is an inherited OPEN gap, not
  introduced here.

---

## Appendix: symbols cited

Verified **present** on this branch (read while writing this):

- `lsp-max-protocol/src/conformance.rs` — `ConformanceVector`
  (`admitted`/`refused`/`unknown`, `set_admitted`/`set_refused`/`set_unknown`,
  `admits_release`, `assert_bitmask_invariants`), `LawAxis::Domain`,
  `LawAxisId::DOMAIN`.
- `lsp-max-runtime/src/mesh.rs` — `AutonomicMesh`, `build_conformance_vector()`.
- `lsp-max-runtime/src/mesh_types.rs` — `Hook`, `MeshAction`
  (`AddDiagnostic`/`EmitReceipt`/`ExecuteBoundedAction`), `MaxMethod::AutonomicLoop`,
  `ConformanceDeltaEntry`.
- `lsp-max-protocol/src/hooks.rs` — `HookEvent`, `AdmissionDecision`
  (`Admitted`/`Refused`/`Unknown`), `AutonomicLoopStatus`.
- `lsp-max-protocol/src/core.rs` — `Receipt` (`prev_receipt_hash`),
  `ReceiptObligation`, `GateId`.
- `lsp-max-protocol/src/diagnostics.rs` — `MaxDiagnostic`, `Repairability::Unknown`.
- `lsp-max-runtime/src/control_plane/receipts.rs` — `CryptographicReceipt`,
  `verify_receipt_chain()`.
- `crates/lsp-max-cli/src/nouns/gate.rs` — `GateService::check` /
  `gate_file_path`, `GateCheckResult`, the `check` verb.
- `examples/conformance_vector_explained.rs`, `examples/receipt_chain_explained.rs`,
  `examples/admission_pipeline.rs` — runnable contract witnesses.

Design **targets** (not on this branch; the intended optimizer shape, §0):

- `PipelineSearchResult`, `PipelineBoundedStatus`
  (`Admitted`/`Partial`/`Unknown`/`Refused`/`Blocked`).
- `diagnostics_for_search()`, `TPOT2_EMPTY_POOL` / `TPOT2_NONCONVERGENCE` /
  `TPOT2_OCEL_MISSING`.
- a `clap-noun-verb` `pipeline` noun + verbs; a TPOT2 marker-receipt emitter and
  validator; a wasm4pm-backed fitness evaluator.
