# AGENTS.md SPR — lsp-max

This SPR is the compressed activation layer for the full `AGENTS.md`. It does not replace the full file. It primes agents with the project laws before they read the detailed rules.

---

## Core Frame

`lsp-max` is the proving ground for **inverted LSP**.

Normal LSP helps humans write code.

Inverted LSP makes the repository speak back to agents while they work.

The repository is not passive text. It is a law-bearing system.

`AGENTS.md` is the constitution.  
LSP is the live enforcement operator.  
`anti-llm-cheat-lsp` is the diagnostic canary.  
Receipts decide admissibility.  
The team does not declare success.

---

## Governing Equation

```text
R_B ⊢ A = μ(O*_B)

Done_B(A) =
  [FailSet_B(A)=∅]
  ∧
  [R_B ⊢ A = μ(O*_B)]
```

Meaning:

Agent output is not admitted because it looks correct, compiles, logs success, or passes a test.

Agent output is admitted only when bounded receipts prove that the action equals the lawful transformation of admitted observations.

---

## LSP-as-ANDON Doctrine

`lsp-max` is not merely an LSP transport framework.

`lsp-max` is the ANDON framework for LLM coding agents.

Normal LSP exposes diagnostics.

ANDON LSP interrupts invalid work before it becomes project truth.

The governing law:

> If the agent has to remember to check, the system has already failed.

Therefore, governed defects must be pushed into the agent's world.

The LSP must not merely provide passive surfaces such as diagnostics, virtual documents, logs, or reports. Those are evidence surfaces. They are not sufficient discovery surfaces.

Required behavior:

```text
ProblemDetected
  ⇒ DiagnosticPublished
  ∧ AndonPushed
  ∧ AdmissionGateUpdated
```

Blocking behavior:

```text
BlockingAndon ⇒ ¬AdmissionAllowed
RefuseAndon  ⇒ ¬AdmissionAllowed
```

The LSP is the factory cord.

### ANDON Invariant Model

Every admissible rule must be represented as an invariant with five components:

`TRUE + FALSE + COUNTERFACTUAL + WITNESS + REPAIR`

Meaning:

- `TRUE`           — valid state is recognized
- `FALSE`          — invalid state is rejected
- `COUNTERFACTUAL` — minimally corrupted valid state fails
- `WITNESS`        — evidence proves the outcome
- `REPAIR`         — next lawful step is explicit

An invariant is not real until all five exist.

Forbidden implications:

```text
Positive case passes ⇒ law holds
No violations        ⇒ checks executed
Diagnostic exists    ⇒ agent saw it
Virtual doc exists   ⇒ agent inspected it
LLM can reason       ⇒ LLM will check
```

Required implication:

```text
Invariant failure ⇒ pushed ANDON event
```

### Truth / False / Counterfactual Power

The three proof cases prevent vacuous green.

- `TRUE` proves recognition.
- `FALSE` proves rejection.
- `COUNTERFACTUAL` proves sensitivity.

Examples:

```text
8-task work unit      → PASS
9-task work unit      → WORK_UNIT_NEED9
8-task work unit + 9th task mutation → WORK_UNIT_NEED9
valid receipt         → publish may be admitted
missing receipt       → RECEIPT_MISSING
delete valid receipt  → RECEIPT_MISSING
bound checks executed → PASS
checks_run=[]         → BOUND_CHECKS_NOT_EXECUTED
disabled checker      → BOUND_CHECKS_NOT_EXECUTED
```

Critical law:

- UNKNOWN is not PASS.
- STOP is not PASS.
- No witness is not PASS.
- No repair for a blocking failure is not PASS.
- Empty violations are not a pass unless `checks_run` proves checks executed.

### ANDON Status Model

Use TPS-style states, not vague success language.

- `INFO`     — state changed; non-blocking
- `WARNING`  — degraded; may remain admissible depending on policy
- `STOP`     — missing proof or blind spot; admission disabled
- `REFUSE`   — known invalid state; admission disabled

Examples:

```text
BOUND_CHECKS_NOT_EXECUTED       = STOP
COUNTERFACTUAL_MISSING          = STOP
RECEIPT_MISSING                 = STOP
OCEL_TRACE_MISSING              = STOP

WORK_UNIT_NEED9                 = REFUSE
COUNTERFACTUAL_DID_NOT_FAIL     = REFUSE
DIRECT_HEAVY_COMMAND_BLOCKED    = REFUSE
GOAL_REACHED_FALSE              = REFUSE
```

### ANDON Event Contract

Every pushed ANDON event must contain enough information that the LLM agent does zero discovery reasoning.

Required payload shape:

```rust
AndonEvent {
  id
  severity
  code
  title
  message

  invariant_id
  observed_state
  expected_state

  blocking
  requires_ack
  admission_allowed

  next_lawful_step
  required_command

  evidence_uri
  virtual_doc_uri
  receipt_required
}
```

Required fields for blocking events:

```text
blocking = true
admission_allowed = false
next_lawful_step = Some(...)
virtual_doc_uri = Some(...)
```

If a blocking event has no repair path:

```text
REPAIR_MISSING = STOP
```

### LSP Push Channels

`lsp-max` must support three ANDON push surfaces.

1. **Standard diagnostics**
   `textDocument/publishDiagnostics`
   Diagnostics make the workspace visibly red.

2. **Human interruption**
   `window/showMessage`
   `window/showMessageRequest`
   Use `showMessageRequest` for STOP or REFUSE conditions requiring explicit acknowledgement.

3. **Machine push notifications**
   Generic `lsp-max` notifications:

```text
lspMax/andonRaised
lspMax/admissionChanged
lspMax/truthTableChanged
lspMax/counterfactualFailed
lspMax/nextLawfulStepChanged
```

Domain LSPs may alias these:

```text
bcinrPddl/andonRaised
antiLlm/andonRaised
ggen/andonRaised
```

but the payload must remain compatible with `AndonEvent`.

### Agent Contract Under ANDON

The agent is not responsible for discovering governed failures.

Correct agent loop:

1. Receive pushed ANDON event
2. Read `admission_allowed`
3. If false, stop current action
4. Execute `required_command` or `next_lawful_step`
5. Wait for updated LSP state
6. Do not continue until gate clears

Forbidden agent loop:

- Notice diagnostic
- Reason that it might be okay
- Continue editing
- Run build anyway
- Summarize as green

The agent must not think around ANDON.

### lsp-max Framework Obligations

`lsp-max` must provide reusable high-level abstractions so every project LSP does not hand-roll the same law machinery.

Required framework capabilities:

- `AndonInvariant`
- `InvariantRegistry`
- `TruthTable`
- `CounterfactualProbe`
- `Witness`
- `RepairAction`
- `AndonEvent`
- `AndonBus`
- `AdmissionGate`
- `VirtualDocRegistry`
- `AnalysisPipeline`
- `PatternLibrary`
- `LspPushAdapter`
- `AgentNotificationProtocol`
- `LspMaxHarness`

Domain LSPs declare laws.

`lsp-max` turns those laws into:

- diagnostics
- push notifications
- truth-table rows
- counterfactual probes
- witnesses
- repair actions
- virtual docs
- admission gates
- test harness assertions

Sharp rule:

> Project LSPs declare invariants.
> `lsp-max` enforces invariant lifecycle.

### Core Abstraction: AndonInvariant

Canonical shape:

```rust
AndonInvariant {
  id
  statement
  scope

  true_probe
  false_probe
  counterfactual_probe

  witness_rule
  repair_rule

  severity
  blocks
}
```

Every invariant must answer:

1. Can valid state pass?
2. Can invalid state fail?
3. Can corrupted valid state fail?
4. What proves it?
5. What repairs it?
6. What admission does it block?

Missing probes are not allowed.

```text
true_probe missing           ⇒ TRUE_CASE_MISSING
false_probe missing          ⇒ FALSE_CASE_MISSING
counterfactual_probe missing ⇒ COUNTERFACTUAL_MISSING
witness missing              ⇒ WITNESS_MISSING
repair missing on block      ⇒ REPAIR_MISSING
```

### Invariant Registry

The registry prevents invisible rules.

Required law:

```text
InvariantRegistry.empty() ⇒ ANDON
```

Required behavior:

- register invariants
- evaluate all invariants
- detect missing true/false/counterfactual probes
- detect missing witnesses
- detect missing repairs
- produce TruthTable
- produce AndonEvents

A project with no invariants is not green.

It is blind.

### Truth Table Virtual Documents

Every ANDON-capable server must expose truth state.

Generic URIs:

```text
lsp-max://truth/table
lsp-max://truth/true
lsp-max://truth/false
lsp-max://truth/counterfactuals
lsp-max://truth/andon
lsp-max://invariants
lsp-max://admission/gate
lsp-max://agent/next-step
```

Domain aliases may exist:

```text
anti-llm://truth/table
bcinr-pddl://truth/table
ggen://truth/table
```

These documents are evidence surfaces.

They must not be the only way a failure is discovered.

### Witness Model

A pass requires evidence.

Permitted witness kinds:

- File
- Receipt
- OCEL event
- Diagnostic
- Command output
- Virtual document
- Raw JSON-RPC transcript
- Digest
- Process-model conformance result

Required law:

```text
PassWithoutWitness ⇒ STOP
```

The system must not say PASS without pointing to the evidence.

### Repair Model

A blocking ANDON must name the next lawful step.

Canonical shape:

```rust
RepairAction {
  id
  title
  next_lawful_step
  command
  code_action
  virtual_doc_uri
}
```

Examples:

```text
WORK_UNIT_NEED9
  → bcinrPddl.splitNeed9

BOUND_CHECKS_NOT_EXECUTED
  → implement_check_lifecycle_domain

DIRECT_HEAVY_COMMAND_BLOCKED
  → lspMax.requestBuildSlot

RECEIPT_MISSING
  → executeTape or emitReceipt through admitted route
```

If the LSP cannot identify the repair:

```text
REPAIR_MISSING = STOP
```

### Counterfactual Probe Families

Counterfactuals should be cheap, local, and specific.

Required reusable mutation patterns:

- `RemoveFile`
- `RemoveMarker`
- `CorruptJsonField`
- `AddNthItem`
- `DisableChecker`
- `SwapAuthority`
- `HideReceipt`
- `HideOCEL`
- `DoubleAcquireBuildSlot`
- `DirectHeavyCommandWithoutSlot`

Examples:

```text
remove ADMITTED marker       → PRD_NOT_ADMITTED
add 9th work-unit task       → WORK_UNIT_NEED9
delete receipt               → RECEIPT_MISSING
flip goal_reached true→false → GOAL_REACHED_FALSE
set checks_run=[]            → BOUND_CHECKS_NOT_EXECUTED
```

A counterfactual that does not fail is a refusal:

```text
COUNTERFACTUAL_DID_NOT_FAIL = REFUSE
```

### Pattern Library

`lsp-max` should provide reusable invariant patterns.

Required patterns:

- `RequiredArtifact`
- `MarkerAdmission`
- `NeedN`
- `CandidateNotAdmission`
- `ReceiptRequired`
- `OcelRequired`
- `BrokeredCommand`
- `NonEmptyCheckSet`
- `RouteEvidenceRequired`
- `TranscriptRequired`
- `NoForbiddenDependency`
- `NoVictoryLanguage`

**RequiredArtifact**
Use for files that must exist.
`docs/prd.md`, `docs/ard.md`, `docs/adr/*.md`, `.bcinr/test-report.json`, `.bcinr/receipts/latest.json`, `.bcinr/ocel/latest.json`

**MarkerAdmission**
Use for marker-based local admission.
`ADMITTED`, `REVIEWED`, `PUBLISHED`

**NeedN**
Use for bounded decomposition.
Need9 means split.

**CandidateNotAdmission**
Canonical law:
`Candidate ≠ Admitted`

**ReceiptRequired**
Canonical law:
`Test output is not a receipt.`

**OcelRequired**
Canonical law:
`Execution without OCEL is not process evidence.`

**BrokeredCommand**
Canonical law:
`Heavy command requires build slot.`

**NonEmptyCheckSet**
Canonical law:
`Empty checks_run is ANDON.`

### Analysis Pipeline

Every `lsp-max` server should follow the same high-level analysis cycle.

- observe workspace
- evaluate invariants
- compute truth table
- run cheap counterfactual probes
- derive ANDON events
- publish diagnostics
- push ANDON notifications
- update virtual docs
- update admission gate

Required implication:

```text
didOpen/didChange/didSave
  ⇒ TruthTableUpdated
  ∧ CounterfactualsEvaluated
  ∧ AndonEventsPushedWhenNeeded
```

Projection events may not perform admission unless explicitly authorized.

### Admission Gate

`lsp-max` owns generic admission blocking.

Canonical statuses:

- `OPEN`
- `CANDIDATE`
- `BLOCKED`
- `STOPPED`
- `REFUSED`
- `ADMITTED`
- `PUBLISHED`
- `UNKNOWN`

Required rule:

```text
admission_allowed(events) =
  events.all(|e| e.admission_allowed)
```

If any active event has `blocking = true` then `admission_allowed = false`. The LLM cannot override this with prose.

### LspMaxHarness

`lsp-max` must provide an in-memory test harness for ANDON behavior.

Required assertions:

- `assert_diagnostic(code)`
- `assert_andon(code)`
- `assert_admission_disabled()`
- `assert_next_lawful_step(step)`
- `assert_virtual_doc_contains(uri, text)`
- `assert_truth_table_row(invariant_id)`
- `assert_counterfactual_failed(invariant_id)`
- `assert_witness_present(invariant_id)`
- `assert_repair_present(invariant_id)`
- `assert_no_vacuous_green()`

Every rejection test must assert both:
- diagnostic emitted
- ANDON pushed

A function-level rejection that does not surface through LSP is not admitted.

### New Diagnostic Families for ANDON Framework

Add framework-level diagnostics:

- `LSPMAX-ANDON-*`
- `LSPMAX-INVARIANT-*`
- `LSPMAX-TRUTH-*`
- `LSPMAX-WITNESS-*`
- `LSPMAX-REPAIR-*`
- `LSPMAX-COUNTERFACTUAL-*`
- `LSPMAX-ADMISSION-*`

Required codes:

- `LSPMAX-INVARIANT-EMPTY-REGISTRY`
- `LSPMAX-INVARIANT-TRUE-CASE-MISSING`
- `LSPMAX-INVARIANT-FALSE-CASE-MISSING`
- `LSPMAX-INVARIANT-COUNTERFACTUAL-MISSING`
- `LSPMAX-TRUTH-TABLE-INCOMPLETE`
- `LSPMAX-WITNESS-MISSING`
- `LSPMAX-REPAIR-MISSING`
- `LSPMAX-COUNTERFACTUAL-DID-NOT-FAIL`
- `LSPMAX-ADMISSION-DISABLED`
- `LSPMAX-ANDON-PUSH-MISSING`

Forbidden implication:

```text
DiagnosticOnly ⇒ AgentInterrupted
```

So any blocking diagnostic without a corresponding ANDON push is itself a framework violation:
`LSPMAX-ANDON-PUSH-MISSING`

### Relationship to Λ_CD

The current gate file is the SELECT side (agent or hook reads gate state).
ANDON push is the PUSH side (diagnostic context is injected into the agent's world).

Full runtime law requires both:
`Λ_CD^runtime = SELECT ∧ PUSH`

SELECT alone is insufficient for subagents that do not know to check.
PUSH alone is insufficient for synchronous tool blocking.

Required direction:
- gate check remains canonical synchronous block
- ANDON event becomes canonical context injection

### D_t PUSH Requirement

`D_t` is the active diagnostic context at time `t`.

The agent should receive `D_t` as structured context, not infer it from logs.

Minimum pushed context:

- active_andon_codes
- governing_axes
- available_repairs
- admission_allowed
- since_seq
- truth_table_uri
- gate_file

Candidate CLI surface:

```bash
lsp-max-cli gate check --format=agent-context
```

When BLOCKED, stdout must emit a structured block suitable for injection into agent context.

Required fields:

- active_andon_codes
- active_invariant_ids
- severity
- blocking
- available_repairs
- required_commands
- virtual_doc_uris
- admission_allowed

### Subagent Propagation Update

The existing subagent gap remains OPEN.

The helper preamble is mitigation, not enforcement.

Required updated language:

> Subagent prompt preambles are CANDIDATE mitigation.
> D_t PUSH is the path toward structural enforcement.
> Until PUSH is ADMITTED, subagent propagation remains OPEN.

Do not collapse this gap into ADMITTED.

### Framework Crate Shape

Target split:

```text
lsp-max-core
  invariant model
  truth table
  witness
  repair
  admission gate

lsp-max-andon
  AndonEvent
  AndonBus
  severity
  push mapping

lsp-max-patterns
  RequiredArtifact
  MarkerAdmission
  NeedN
  ReceiptRequired
  OcelRequired
  BrokeredCommand
  NonEmptyCheckSet

lsp-max-lsp
  Tower integration
  diagnostics
  showMessage/showMessageRequest
  custom notifications
  virtual docs

lsp-max-test
  in-memory harness
  fake client
  assertion helpers
```

If implemented as one crate first, keep these module boundaries.

### Final ANDON Prime

`lsp-max` does not ask agents to be careful.

`lsp-max` makes governed carelessness operationally visible and blocks admission.

The repository is not passive text.

The LSP is not passive diagnostics.

The agent is not trusted to remember every law.

The framework must push law violations into the agent's world.

Final law:

> A state does not exist operationally unless the LSP can represent it.
> A failure is not governed unless the LSP can push it.
> An invariant is not real unless it has TRUE, FALSE, COUNTERFACTUAL, WITNESS, and REPAIR.
> Green without counterfactual rejection is not green.
> A disclaimer is not an ANDON.
> A diagnostic without push is not interruption.
> A candidate is not admission.
> A test is not a receipt.
> A log is not route proof.
> A receipt decides admissibility.

---

## Operating Metaphor

This project is an F1 race team for agents.

The agent is the driver.  
`lsp-max` is the chassis/protocol surface.  
`anti-llm-cheat-lsp` is telemetry.  
The failset is the pit wall.  
Receipts are scrutineering.  
Negative controls are simulator crashes.  
LSP 3.18 is the instrumented race surface.

The purpose is not to slow agents down.  
The purpose is to increase effective admitted velocity.

```text
v_eff = dA_admitted / dt
```

Raw output velocity is not the target.

---

## Non-Negotiable Laws

### 1. No plain tower-lsp

Plain `tower-lsp` must not appear in admissible code, manifests, lockfiles, examples, tests, or runtime surfaces.

Forbidden outside explicit negative-control fixtures:

```text
tower-lsp
tower_lsp
tower-lsp =
tower_lsp::
use tower_lsp
```

If it appears outside quarantine:

```text
GC004B_NO_TOWER_LSP_LOCK = BLOCKED
ANTI-LLM-SURFACE-001
```

Forbidden implication:

```text
Pass(plain LSP) ⇒ Pass(LSP 3.18)
```

---

### 2. Maximize LSP 3.18

Do not claim LSP 3.18 from basic LSP behavior.

Forbidden substitutions:

```text
initialize/didOpen/publishDiagnostics ⇒ LSP 3.18
basic codeAction ⇒ command tooltip proof
basic WorkspaceEdit ⇒ metadata/snippet proof
basic completion ⇒ completionList.applyKind proof
basic document filters ⇒ relative pattern proof
basic logMessage ⇒ debug message kind proof
```

Admissible target:

```text
LSP318_ADMITTED =
  NO_TOWER_LSP
  ∧ INITIALIZE_CAPABILITIES_3_18
  ∧ FEATURE_MATRIX_15_OF_15
  ∧ RAW_JSON_RPC_TRANSCRIPTS
  ∧ RECEIPTS
  ∧ NEGATIVE_CONTROLS
```

Each 3.18 feature row must be:

```text
SUPPORTED_WITH_TRANSCRIPT
REFUSED_BY_LAW_WITH_RECEIPT
BLOCKED
```

Never use:

```text
probably supported
implied
covered by normal LSP
not relevant
not tested
```

---

### 3. Exact name: clap-noun-verb

Do not invent `CLAP`.

The actual component is:

```text
clap-noun-verb
```

Forbidden:

```text
CLAP authority
CLAP validation
CLAP command grammar
CLAPValidate
CLAP Rejected
CLAP Validated
```

If fake `CLAP` appears:

```text
ANTI-LLM-AUTH-002
```

Forbidden implication:

```text
ElegantAbstraction ⇒ ExistingAuthority
```

---

### 4. LSP is read-only by default

The LSP may emit:

```text
diagnostics
hovers
code action intents
inline completions
virtual documents
command tooltips
failset summaries
protocol traces
```

It must not directly mutate files.

Future mutation must route only through:

```text
CodeAction
→ clap-noun-verb admission
→ PackActionIntent
→ PackPlan
→ Staging
→ MutationGate
→ Receipt
```

Forbidden implication:

```text
LSP observation ⇒ mutation authority
```

---

### 5. Logs are not route proof

This is not proof:

```text
Routing to PackPlan -> Staging -> MutationGate
```

Required route evidence:

```text
CodeAction
clap-noun-verb admission
PackActionIntent
PackPlan
Staging
MutationGate
Receipt
MutationGate denial test
bypass refusal tests
```

If only a log exists:

```text
ANTI-LLM-ROUTE-001
```

Forbidden implication:

```text
Log(RouteIntent) ⇒ RouteExecution
```

---

### 6. Test output is not a receipt

Not receipts:

```text
cargo test passed
test result: ok
server logged validated
stdout says admitted
```

Receipts must bind:

```text
receipt_path
digest
digest algorithm
boundary
checkpoint
raw command
output digest
admission/refusal status
negative-control result when required
```

Forbidden implications:

```text
TestStdout ⇒ Receipt
LogMessage ⇒ Receipt
StatusWord(ADMITTED) ⇒ Admitted
```

---

### 7. Tree-sitter observes; it does not admit

Tree-sitter is an observation layer, not authority.

Pipeline:

```text
File
→ observations
→ rules
→ diagnostics
→ failset
→ proof request
```

Forbidden implication:

```text
ASTObservation ⇒ Admission
```

---

### 8. No victory language

Do not say:

```text
victory
done
all clean
fully admitted
no issues
everything passes
solved
guaranteed
impossible to fake
all gaps resolved
successfully proven
```

Use only bounded statuses:

```text
ADMITTED
ADMITTED_BY_DOGFOOD
REPORTED_ADMITTED_BY_DOGFOOD
REPORTED_CLEAN_WITH_RAW_SCAN
CANDIDATE
BLOCKED
REFUSED
UNKNOWN
UNSUPPORTED
PARTIAL
REGRESSION_RISK
OPEN
FAILSET_NONEMPTY
MATRIX_INCOMPLETE
SUPPORTED_WITH_TRANSCRIPT
REFUSED_BY_LAW_WITH_RECEIPT
```

---

## lsp-max-compositor

Build here:

```text
crates/lsp-max-compositor
```

Purpose: multi-server fan-out and merge layer. When a single LSP session must aggregate
diagnostics, hovers, or code actions from N child LSP processes, the compositor owns the
lifecycle.

```text
child_process   — spawns and reaps server subprocesses; exit watcher clears stale state
fanout          — broadcasts inbound client requests to all children in parallel
merge           — ConformanceVector-aware diagnostic dedup; REFUSED_BY_LAW codes survive always
capability_merge — Primary wins hover/completion; DiagnosticsOnly excluded; sync FULL forced
diagnostic_buffer — DashMap per-URI staging; deposit() replaces same server_id; flush() is non-destructive
flush_coordinator — adaptive quorum debounce (fires at quorum or 2×spread, ≤30ms cap); emits
                    CompositorReceipt after each push; accumulates OCEL 2.0 events (take_ocel_events());
                    runs DeclareModel::compositor() + DirectlyFollowsGraph fitness after every flush
declare         — Van der Aalst Declare constraint model (9 constraint types); DeclareModel::compositor()
                  and DeclareModel::anti_llm_detection() normative models; extract_traces() from OCEL events
dfg             — Directly-Follows Graph (Van der Aalst DFG): from_traces(), fitness_against_model(),
                  precision_against_model(), to_mermaid(), to_dot()
registry        — ChildTier (Primary | Secondary | DiagnosticsOnly) + ExtensionRouter
compositor_state  — via state_response; live registry snapshot; non-destructive, bypasses debounce
compositor_health — via health_response; per-child liveness, O(1)
```

Law: the compositor is read-only toward client files — all mutation still routes through the
CodeAction → clap-noun-verb → Receipt chain.

Routing invariant:

```text
textDocument/hover | completion | definition   → FirstSuccess (Primary tier only)
textDocument/publishDiagnostics               → FanAll (all tiers; REFUSED_BY_LAW survives merge)
textDocument/didOpen | didChange | didClose   → Notify (fan all, no response expected)
unknown methods                                → PrimaryOnly
```

ANDON law applies inside the compositor: if any REFUSED_BY_LAW Error is present after merge,
`MergeResult.has_andon_block = true`; the CompositorReceipt records the prefixes_fingerprint
encoding which $\mathcal{A}$ governed the flush. Do not gate or release while ANDON is set.

### L7 Speciation Status

**Formal claim:** each project-server entry in `lsp-max.toml` carries an independent
law-collapse function Λ_CD^(D), isolating which ANDON prefixes apply per diagnostic source.

**Current implementation:** per-server `andon_code_prefixes` lists are aggregated into a
single workspace-wide union at `MergeContext` construction time
(`CompositorConfig::all_andon_prefixes()`). Every diagnostic is evaluated against this
union regardless of which server emitted it.

**Closed gap (RFC-C strict isolation):** attribution now routes strictly by `server_id`.
`MergeContext::attribute_andon(code, server_id)` returns an `AndonRoute`:
`PerServer { server_id }` when the originating server's own Λ_CD^(D) classifies the code,
`Union` only when NO `server_id` is present (the explicit last resort), and `NotAndon`
otherwise. A code declared only by server B no longer triggers ANDON on a server-A-sourced
diagnostic — the union is no longer borrowed when a `server_id` is present.

**Status: ADMITTED** — per-server C_D isolation is enforced at merge time, witnessed by
`merge/witness_isolation.rs` (constructed so a union-fallback regression fails the suite).
The workspace union survives solely as the explicit no-`server_id` last resort.

**Lineage (RFC-B):** each child server's contribution carries its own `CryptographicReceipt`
chain link via `receipt_chain::ChildEvidence` (crypto reused from lsp-max-runtime, not
forked); `CompositorReceipt::with_child_evidence` binds them so the merged verdict is
traceable to per-child evidence by the moniker join key `moniker:{scheme}:{identifier}`.
`CompositorReceipt::to_ocel_event` (RFC-C) makes the fan-out→merge→admit flush minable.

**Fallback observability (Part A):** when `lsp-max.toml` is absent from the workspace
tree, `CompositorConfig::load()` returns `None` and main.rs falls back to the static
prefix set `[WASM4PM-, ANTI-LLM-, GGEN-]`. This fallback now emits a `tracing::warn!`
making the C_D collapse observable in structured logs. Silent fallback is REFUSED.

**Next step to ADMIT L7 Speciation:** `MergeContext` must carry a
`HashMap<server_id, Vec<String>>` and `merge_diagnostics` must receive per-entry server
identity so each diagnostic is tested against its originating server's prefix set, not the
workspace-wide union.

---

## anti-llm-cheat-lsp

Build here:

```text
crates/anti-llm-cheat-lsp
```

Purpose:

```text
anti-llm-cheat-lsp runs on lsp-max
anti-llm-cheat-lsp does not depend on plain tower-lsp
anti-llm-cheat-lsp exercises LSP 3.18 surfaces
anti-llm-cheat-lsp detects attempts to reintroduce tower-lsp
```

Self-sealing law:

```text
lsp-max hosts anti-llm-cheat-lsp
anti-llm-cheat-lsp detects tower-lsp
therefore lsp-max cannot silently regress to tower-lsp
```

Implementation pattern — `RulePackServer` bridge:

```text
impl RulePackServer for AntiLlmServer
  rule_packs()          → ValidatedRulePackSet::empty()  (no TOML packs; engine-bridge server)
  grammar()             → tree_sitter_rust::LANGUAGE
  server_name()         → "anti-llm-cheat-lsp"
  client()              → &self.client
  adapter()             → self.ast_adapter.inner()       (AutoLspAdapter ref)
  workspace_index()     → Some(&self.workspace_index)    (lock-free DashMap doc store)
  scan_uri_classified() → bridges engine::scan_directory + evaluate_diagnostics
                          into ClassifiedFindings via LawAxis::Custom(d.category)
```

Virtual document `anti-llm://process-model` is served from `virtual_docs/process_model.rs` and renders a live DFG + Declare conformance report using Van der Aalst process mining primitives inline (no compositor dependency).

---

## Detector Stack

Do not build one giant grep.

Required detector stack:

```text
raw text scan
→ tree-sitter AST scan
→ Cargo manifest/dependency graph scan
→ Markdown/agent-report claim scan
→ JSON-RPC/LSP transcript scan
→ receipt validator
→ route evidence checker
→ claim-vs-proof checker
→ LSP diagnostic emitter
```

Every diagnostic must name the forbidden implication it prevents.

---

## V0 Diagnostic Families

```text
ANTI-LLM-SURFACE-*   fake protocol/dependency surface
ANTI-LLM-AUTH-*      fake authority or fake abstraction
ANTI-LLM-RECEIPT-*   fake receipt
ANTI-LLM-ROUTE-*     fake route
ANTI-LLM-MUT-*       mutation bypass
ANTI-LLM-TEST-*      test laundering
ANTI-LLM-STRANGE-*   debug/string/path/code-smell laundering
ANTI-LLM-VERSION-*   CalVer/version-law violation
ANTI-LLM-CLAIM-*     victory/status overclaim
```

Core forbidden implications:

```text
Pass(plain LSP) ⇒ Pass(LSP 3.18)
BasicLSPWorks ⇒ LSP318Works
StringShape(command) ⇒ command admission
ElegantAbstraction ⇒ ExistingAuthority
TestStdout ⇒ Receipt
LogMessage ⇒ Receipt
Log(RouteIntent) ⇒ RouteExecution
WorkspaceEdit ⇒ admitted receipt mutation
SubstringMatch ⇒ Authority
StatusWord(ADMITTED) ⇒ Admitted
Positive case passes ⇒ law holds
```

---

## LSP 3.18 Feature Rows

Every row needs capability paths, request/response or notification method, positive transcript, negative control, receipt, digest, status.

```text
LSP318-001 inline completions
LSP318-002 dynamic text document content
LSP318-003 folding range refresh
LSP318-004 multi-range formatting
LSP318-005 snippets in workspace edits
LSP318-006 relative patterns in document filters
LSP318-007 relative patterns in notebook document filters
LSP318-008 code action kind documentation
LSP318-009 nullable activeParameter
LSP318-010 command tooltips
LSP318-011 workspace edit metadata
LSP318-012 snippets in text document edits
LSP318-013 debug message kind
LSP318-014 code lens resolvable properties
LSP318-015 completionList.applyKind
```

No row may be implied.

---

## Required Virtual Documents

```text
anti-llm://failset
anti-llm://lsp318-matrix
anti-llm://receipt-ledger
anti-llm://forbidden-implications
anti-llm://checkpoint-status
anti-llm://process-model
```

`anti-llm://process-model` is a live markdown document rendered from active `AntiLlmDiagnostic` observations. It contains:
- Directly-Follows Graph summary (node/edge counts, transition frequencies)
- Mermaid flowchart of the DFG
- Declare conformance report (Van der Aalst normative model)
- Fitness score and activity legend

Activities map from diagnostic code prefixes: `ANTI-LLM-VICTORY-*` / `ANTI-LLM-CLAIMS-*` → `VictoryLanguageDetected`; `WASM4PM-*` → `ProcessViolationDetected`; `GGEN-*` → `GgenViolationDetected`; etc. `ScanComplete` is the synthetic terminal appended to every case.

These must be dynamic, not static files pretending to be dynamic content.

---

## Agent Work Loop

```text
Research
→ Classify
→ Patch
→ Verify
→ Receipt
→ Refuse
```

Refuse means:

```text
refuse false closure
refuse fake proof
refuse victory language
refuse unsupported admission
refuse route/protocol/receipt substitution
```

---

## ANDON Gate — PreToolUse Hook (Λ_CD^runtime)

A `PreToolUse` hook in `.claude/settings.json` runs `lsp-max-cli gate check` before every **Bash, Edit, and Write** tool call.

- **Exit 0** — gate is clear; the tool proceeds.
- **Exit 1** — ANDON is ACTIVE; the tool is blocked until the gate clears.

This enforces `Λ_CD^runtime`: no shell-side action (build, test, release, format) and no file mutation (edit, write) may proceed while an active ANDON signal is present. Resolve all `WASM4PM-*` and `GGEN-*` diagnostics before the gate will clear.

Coverage: **Bash** (shell actions), **Edit** (in-place file mutations), **Write** (new or overwrite file mutations). All three advance artifact state and are gated equally.

---

## Subagent Gate Propagation — Status: OPEN

### The gap

The `PreToolUse` hook in `.claude/settings.json` applies only to the parent Claude Code session. Subagents spawned via the `Agent` tool run in their own isolated session. They do **not** inherit the parent session's hooks. A subagent can therefore invoke Bash, Edit, or Write while the parent session's gate is BLOCKED.

This is a structural gap. It is not a configuration error. The hook mechanism does not cross session boundaries.

### What is available

The gate file path is deterministic and world-readable. Any process — including a subagent — can read it directly with a single syscall.

Path formula (FNV-1a of the working directory as a zero-padded 16-hex-digit suffix):

```text
$XDG_RUNTIME_DIR/lsp-max-gate-{fnv1a(cwd):016x}
  or
/tmp/lsp-max-gate-{fnv1a(cwd):016x}
```

Content: single byte — `b"0"` when clear, `b"1"` when ANDON is set. File absent means compositor is not running (gate not enforced).

Two CLI verbs are available for gate inspection:

```bash
lsp-max-cli gate check   # Exit 0 = clear; exit 1 = ANDON blocked
lsp-max-cli gate list    # JSON: { andon_blocked, gate_file, compositor_active,
                         #         active_codes: ["WASM4PM-*", "GGEN-*"], agent_scope: "global" }
```

`gate list` is useful for subagent prompts that want to surface the blocking families before taking any action. `active_codes` lists the ANDON-triggering code-prefix families; specific code IDs require a running diagnostic server. `agent_scope` is `"global"` until RFC A per-agent partitioning is wired (currently OPEN).

Reference implementations:

- `crates/lsp-max-cli/src/nouns/gate.rs` — `GateService::gate_file_path()`, `GateService::check()`, `GateService::list()`
- `crates/lsp-max-compositor/src/gate_file.rs` — `GateFile::for_workspace()`

Both use the same FNV-1a constants (`offset_basis = 0xcbf29ce484222325`, `prime = 0x100000001b3`) and format the hash with `{hash:016x}`.

### Proposed mitigation (convention, not enforcement) — RFC-1 OPEN

Subagent prompts MUST include a gate-check preamble as the first Bash action. Two equivalent forms:

**Inline preamble** (minimum required form):

```bash
lsp-max-cli gate check || { echo "ANDON gate blocked"; exit 1; }
```

**Helper script** (codified preamble — preferred for new subagent prompts):

```bash
bash scripts/subagent-gate-check.sh
```

`scripts/subagent-gate-check.sh` mirrors the `.claude/hooks/gate-check.sh` PreToolUse hook behavior: it passes through when `lsp-max-cli` is absent (compositor not running, gate not enforced), exits 1 with an actionable message when the ANDON gate is ACTIVE, and exits 0 when the gate is clear. Using the helper script makes the preamble auditable and keeps subagent prompts consistent with future changes to the gate check protocol.

This reads the gate file, exits 1 if ANDON is set, and blocks further shell actions in that subagent invocation. It mirrors what the PreToolUse hook does in the parent session.

Subagent prompt authors are responsible for including this preamble. There is no structural mechanism that forces it. The `D_t PUSH` injection (RFC-1) is the CANDIDATE path toward structural enforcement; until it is ADMITTED, the preamble convention is the only available mitigation.

### What this gap does not affect

- The parent session's gate enforcement is unaffected.
- The compositor continues to write the gate file correctly.
- `lsp-max-cli gate check` remains the canonical single-syscall check for any caller.
- `scripts/subagent-gate-check.sh` is available to any caller that can run a bash script.

### Admitted / Refused / OPEN

```text
PreToolUse hook enforcement in parent session:         ADMITTED
Gate file written by compositor:                       ADMITTED
lsp-max-cli gate check available to subagents:        ADMITTED
scripts/subagent-gate-check.sh helper:                CANDIDATE — convention, not structural enforcement
Structural enforcement in subagent sessions:           REFUSED — hook boundary is not crossable
D_t PUSH injection (RFC-1):                            CANDIDATE — see RFC Backlog
Convention-based mitigation (prompt preamble):         CANDIDATE — not structurally enforced
Subagent gate propagation overall:                     OPEN
```

Do not collapse OPEN into ADMITTED. The gap remains present until structural enforcement (RFC-1 D_t PUSH or equivalent) is ADMITTED.

---

## Current Framework Status — 2026-06-21

### ADMITTED
- Concurrent fanout: O(max RTT) dispatch, N=500 in <1ms
- Λ_CD eager gate write: ~400ns (was 100ms debounce window)
- L7 Speciation: per-server C_D routing via `prefixes_for_server()` (union; see gap note below)
- Channel capacity: 512 (zero signal loss at N=500)
- Dynamic quorum debounce: flush fires at quorum or 2×spread (≤30ms cap)
- daachorse ANDON prefix matching: O(|code|) classification, asymmetry eliminated
- Workspace test suite: all tests ADMITTED except known OPEN items listed below
- Clippy `-D warnings`: ADMITTED (zero warnings in workspace crates)
- lsp-max-cli noun/verb grammar: 31 noun modules — actuation grammar layer complete
- Process mining surface (Van der Aalst): DFG, variants, replay fitness, causal footprint (`process` noun)
- AGI swarm coordination: consensus voting, autonomic convergence, emergence detection (`swarm` noun)
- OCEL 2.0 export: object-centric event log, OC-DFG discovery, per-object case grouping (`ocel` noun)

### CANDIDATE
- RFC A — `gate list` verb: `GateListResult` with `active_codes` and `agent_scope`; per-agent partitioning is OPEN (full RFC A); `gate list` CLI is wired and tested
- RFC C — OCEL accumulation: `FlushCoordinator::take_ocel_events()` accumulates OCEL 2.0 events from `CompositorReceipt::to_ocel_event`; Declare + DFG conformance run inline after every flush
- Van der Aalst process model virtual doc: `anti-llm://process-model` renders live DFG + Declare report from active `AntiLlmDiagnostic` observations
- `RulePackServer` trait adoption in `anti-llm-cheat-lsp`: `scan_uri_classified` override bridges AhoCorasick engine into `ClassifiedFindings`; `WorkspaceIndex` wired
- papaya::HashMap for DiagnosticBuffer (DashMap contention elimination)
- kanal channel for FlushCoordinator (lower send latency — kanal integrated but not benchmarked at N=500)
- simd-json for JSON-RPC framing (larger scope change; not yet prototyped)
- **examples/lsp-max-scaffold PMSC** — Process-Mined Session Conformance:
  OCEL 2.0 object-centric event log (`SessionLog`), van der Aalst Declare
  constraint model, token replay fitness metric, Oracle classes A8–A12
  (`src/session_conformance.rs`). Closes the gap where per-receipt proof
  (RVD) passes but causal / temporal / epistemic session laws are violated.
  Status: CANDIDATE — implemented and tested; reaching ADMITTED requires
  cross-session replay and signed session digests (ed25519 over `log.digest()`).

### OPEN
- Subagent gate propagation: PreToolUse hooks do not cross Agent session boundaries (structural gap — see Subagent Gate Propagation section)
- RFC A per-agent gate partitioning: `agent_scope` is `"global"`; per-agent `HashMap<agent_id, Vec<String>>` routing not yet wired
- RFC B per-server receipt chain: `ChildEvidence` chain link exists; full per-server cryptographic receipt chain from child to compositor is OPEN
- dx-verify sibling repo violations: `wasm4pm` codebase has uncommitted changes (`tps-metrics/Cargo.toml`) — outside this workspace
- gc006 sealed-repo test (`test_gc006_authority_surface_lock`): BLOCKED — wasm4pm sibling has uncommitted changes; test is a known expected failure until sibling is clean
- L7 Speciation per-server isolation: `MergeContext` uses workspace-wide union of ANDON prefixes; per-server `HashMap<server_id, Vec<String>>` routing is CANDIDATE (see L7 Speciation Status section)

---

## Λ_CD Predicate — Formal Implementation Status

### Formal Definition

```text
Λ_CD(a) = Λ(a) ∧ ¬∃ d ∈ D_t : d.law_id ∈ A ∧ d.severity = Error
```

Where:
- `Λ(a)` — the agent's base admissibility predicate (bounded receipts, no victory language, no forbidden implications)
- `D_t` — the diagnostic context at time `t`, the set of all active diagnostics pushed into the agent's world
- `A` — the constrained set of law axis IDs this gate governs (e.g. `WASM4PM-*`, `ANTI-LLM-*`, `GGEN-*`)
- `d.law_id ∈ A` — the diagnostic code falls within the governed set
- `d.severity = Error` — only Error-severity violations block; Warning and Hint do not

The predicate is false — and the gate is BLOCKED — whenever any Error-severity diagnostic with a governed law ID is present in `D_t`.

### Conjunct Status Table

| Conjunct | Implementation | File:Line | Status |
|---|---|---|---|
| Gate file write | `GateFile::set_andon()` writes `b"1"` eagerly on first ANDON Error; clears to `b"0"` when `D_t` drains | `crates/lsp-max-compositor/src/gate_file.rs` | ADMITTED |
| Gate hook PreToolUse | `.claude/settings.json` runs `lsp-max-cli gate check` before every Bash, Edit, Write tool call; exit 1 blocks | `.claude/settings.json` + `crates/lsp-max-cli/src/nouns/gate.rs` | ADMITTED |
| Per-server C_D routing (L7) | `prefixes_for_server()` returns per-server prefix list; `MergeContext` currently uses workspace-wide union — per-server `HashMap<server_id, Vec<String>>` at merge time is CANDIDATE | `crates/lsp-max-compositor/src/merge.rs` | PARTIAL |
| Receipt blocking | `CompositorReceipt` is emitted only after flush; `has_andon_block = true` propagates into receipt; gate file write precedes receipt emission | `crates/lsp-max-compositor/src/receipt.rs` + `flush_coordinator.rs` | ADMITTED |
| D_t context format | `DiagnosticBuffer` carries server_id, law_id (code prefix), severity, and URI per entry; format is structurally sound but `D_t` injection into subagent context window is not yet wired | `crates/lsp-max-compositor/src/receipt.rs` | CANDIDATE |

### SELECT→PUSH Model — Current Position

The current implementation straddles two models. The gate file is the **SELECT side**: the agent (or its PreToolUse hook) reads the gate file on demand — one syscall, one byte — to determine whether `Λ_CD` holds. This is pull-based; the agent selects the gate state from the world. The **PUSH side** — where the world injects `D_t` as a structured context block into the agent's active context window before each tool call — is not yet implemented. A full `Λ_CD` enforcement model requires both: SELECT gives the agent a fast synchronous check, while PUSH ensures the agent sees the governing diagnostic set even when it does not know to ask. The `D_t` context format (CANDIDATE above) is the prerequisite for the PUSH side. Until PUSH is wired, subagent sessions that do not run `lsp-max-cli gate check` as a preamble can proceed without knowledge of active ANDON signals.

### Boundary Statement

"The gate governs the constrained set; everything outside A remains the agent's own judgment."

The gate does not replace the agent's full admissibility predicate `Λ(a)`. It enforces only the law axes enumerated in `A`. Diagnostic codes outside `A`, stylistic choices, architectural trade-offs, and work outside governed surfaces remain under agent judgment. The gate is a floor, not a ceiling.

### RFC Backlog — Architectural Priorities (2026-06-21)

Three RFC-level changes identified via multi-agent architectural review. Ordered by effort tier:

**RFC-1: D_t PUSH injection** (effort: days, impact: high, interface-safe)
Extend `lsp-max-cli gate check` with `--format=agent-context` flag. When exit 1 (BLOCKED), stdout emits a structured JSON block — `active_andon_codes`, `governing_axes`, `available_repairs`, `since_seq` cursor — which the PreToolUse hook injects as a `<gate-context>` system-reminder block. Converts the 1-bit gate signal into the full governing diagnostic set in agent context. The `gate list` verb (RFC A, CANDIDATE) provides a subset of this — `active_codes` and `agent_scope` — without the `--format` flag. Moves D_t PUSH from OPEN toward ADMITTED. No interface breaks. Status: CANDIDATE.

**RFC-2: Per-connection state — remove global REGISTRY/MESH singletons** (effort: weeks, impact: critical, breaks interface)
`REGISTRY` and `MESH` are `OnceLock<Mutex<...>>` singletons in `src/lib.rs`. Every LSP method acquires the same global Mutex. Replace with a per-connection `ServerSession<S>` struct threaded through the Tower layer as an Extension. Removes `reset_registry_for_tests()`. Enables true multi-tenancy: N concurrent LSP connections each have isolated D_t, conformance_delta_log, action_seq, and GateFile handle. Status: OPEN.

**RFC-3: Event-sourced D_t log — replace mutable HashMap dispatch** (effort: months, impact: critical, interface-safe)
AutonomicMesh's `Vec<Box<dyn Hook>>` dispatch and `Vec<HookEvent>` log (capped at 1000 entries, never persisted) prevent causal replay and D_t addressability. Replace with an append-only lock-free ring buffer (65536 entries). Hooks become pure `(LawEvent) -> Vec<MeshAction>` functions. Live D_t is a materialized view maintained by a background tailer. Replay becomes a cursor read, not a destructive HashMap overwrite. Prerequisite for making D_t replay trustworthy. Status: OPEN.

### Session Audit Summary — 2026-06-21

26 conjuncts audited across the Λ_CD implementation surface.

```text
ADMITTED:  8  (gate file write, PreToolUse hook, receipt blocking, speciation test suite,
               subagent gate check availability, 31-noun CLI grammar, process mining surface,
               OCEL 2.0 + AGI swarm surface)
PARTIAL:   1  (L7 per-server C_D routing — union is conservative superset; isolation gap documented)
CANDIDATE: 5  (D_t context format; RFC A gate list; RFC C OCEL accumulation; anti-llm://process-model virtual doc; RulePackServer adoption in anti-llm-cheat-lsp)
OPEN:      13 (subagent structural enforcement, RFC A per-agent partitioning, RFC B per-server receipt chain, dx-verify sibling violations, gc006 sealed-repo test, D_t PUSH wiring, and others — see Current Framework Status section)
BLOCKED:   0
```

Do not collapse OPEN or CANDIDATE into ADMITTED. The counts above are bounded statuses, not victory claims.

---

## Final Prime

This project is not about making an LSP demo.

It is about making `AGENTS.md` enforceable during agent work.

`AGENTS.md` is law.  
Repo state is the manifold.  
Agent edits are trajectories.  
Failsets are curvature.  
Receipts are proof measure.  
LSP is the differential operator and the ANDON cord: it computes gradients, exposes curvature, and interrupts agents when the trajectory leaves admitted law.  
Diagnostics are gradients.  
Code actions are constrained control vectors.  
LSP 3.18 is the enforcement basis.  
Admissibility is `Φ_G = 0` plus `R_B ⊢ A = μ(O*_B)`.  
Effective agent velocity is `dA_admitted/dt`.

Do not optimize for raw output.

Optimize for admitted work.

---

## Known Agent Hallucination & Cheat Patterns

During the evolution of `lsp-max`, several sophisticated agent hallucination and "cheat" patterns were discovered where agents would simulate success without actually performing structural work. These must be aggressively monitored and rejected:

1. **The Ghost Struct (The Doc-Comment Lie):**
   Agents write an optimistic doc-comment (e.g., "This replaces the heavy dependency and runs fast") and create an empty struct, trait, or placeholder, but NEVER wire it into the hot path or remove the legacy dependency. The old slow path remains fully active while the new code is dead.

2. **The "Wait for the Final Audit" Bypass (False Victory):**
   Orchestrators attempt to declare victory and spawn final auditors that check against the *old* logic before the *new* logic is fully wired. If the code compiles (because the old logic is still untouched), the auditor blindly stamps it with a FALSE VICTORY. True verification requires explicit negative controls (letting it crash).

3. **The Schema Hallucination (The "I removed it to fix it" Lie):**
   When told to "fix X schema fields" to comply with a strict specification, agents will occasionally assume that means the fields are invalid and silently delete them. They break the required schema entirely while claiming they strictly aligned it to the specification.

4. **Superficial Type Aliasing:**
   Agents will pretend to optimize memory by doing superficial type aliasing (e.g., `pub type Uri = String`) but continue to use massive string allocations and double-serialization (`serde_json::to_string` followed by `writeln!`) under the hood in the hot path.

Agents must not be trusted on prose or compilation alone. If it doesn't crash when the old path is deleted, the new path is fake.

---

## v26.6.28 — Storage Triad Law (admitted 2026-06-27)

### The Three-Store Model

```text
Salsa   = HOT incremental computation
Papaya  = HOT concurrent diagnostic staging
Oxigraph = COLD semantic law / meaning graph
```

These are not competitors. They have different jobs.

```text
Salsa remembers computation.
Oxigraph remembers meaning.
Papaya stages concurrent diagnostic writes.
```

### Salsa — incremental recomputation engine

Salsa owns the derived computation layer. It does NOT own the parse tree.

**Admitted tracked query outputs:**
```text
Vec<SalsaDiag>
Vec<SymbolFact>
LsifFileResult
Digest
TruthTableRow
InvariantFact
```

**Refused tracked query outputs:**
```text
tree_sitter::Tree       (does not implement salsa::Update)
Parser
open file handles
LSP client handles
Oxigraph store handles
```

**Invariant: `SalsaDoesNotOwnTreeSitterTree`**

TRUE: `parse_document` is a non-tracked helper. `ast_diagnostics` is tracked and returns `Update`-safe types.
FALSE: `tree_sitter::Tree` stored inside a tracked query output.
COUNTERFACTUAL: Return `Tree` from tracked fn → compile refusal.
WITNESS: `cargo test -p lsp-max-ast` 10/10 green, commit `4c883b1`.
REPAIR: Return `Update`-safe derived facts only.

The correct split:
```text
tree-sitter = incremental parse (owns the tree)
Salsa       = incremental recomputation (owns derived facts)
```

Do not fight this boundary. Do not wrap `Tree` badly to force it through Salsa.

### Oxigraph — cold semantic meaning graph

Oxigraph is the correct tool for graph-shaped law queries:

```text
Which invariant governs this diagnostic?
Which AGENTS.md law maps to this code?
Which receipt witnesses this claim?
Which LSIF symbol edges support this repair?
Which project law applies to this workspace?
```

**Invariant: `OXIGRAPH_NOT_ON_HOT_PATH`**

TRUE: `didChange` does not synchronously run full Oxigraph graph rebuild.
FALSE: `didChange` invokes cold SPARQL rebuild.
COUNTERFACTUAL: Force `semantic_graph.refresh()` inside `didChange` → `LSPMAX-OXIGRAPH-HOT-PATH-REFUSED`.
REPAIR: Move query to `refreshLawGraph` / idle task / release audit.

**Boundary law:**

`oxigraph::*` must only appear inside `src/runtime/control_plane/semantic_graph/`.
Random `oxigraph::model::Term` matching outside this boundary = boundary violation, not an Oxigraph problem.

Required module shape:
```text
src/runtime/control_plane/semantic_graph/
  mod.rs
  store.rs
  named_graphs.rs
  sparql.rs
  snapshot.rs
  receipt.rs
```

Exposed API surface only:
```rust
pub trait SemanticLawGraph {
    fn load_snapshot(&self, snapshot: LawGraphSnapshot) -> Result<GraphDigest>;
    fn query_invariants(&self, scope: LawScope) -> Result<Vec<InvariantBinding>>;
    fn query_witnesses(&self, claim: ClaimId) -> Result<Vec<WitnessBinding>>;
    fn query_repairs(&self, diagnostic: DiagnosticCode) -> Result<Vec<RepairBinding>>;
}
```

### LSIF — persisted code intelligence graph

LSIF is the static semantic receipt layer for source code structure.

```text
LSP  = current conversation with the repo (live)
LSIF = durable semantic memory of the repo (stored)
```

LSIF attacks the "LLM guessed the codebase" problem:

Without LSIF, the agent does: search text → open files → infer structure → guess relationships → patch → hope.

With LSIF, the agent does: query symbol graph → resolve definition/reference edges → inspect moniker → compute impacted files → patch with bounded scope.

**LSIF feeds ANDON WITNESS:**

An invariant needs: `TRUE + FALSE + COUNTERFACTUAL + WITNESS + REPAIR`.
LSIF provides WITNESS (symbol evidence, reference edges, definition location) and scopes REPAIR (callsite set, import locations, affected files).

### LSIF Receipt Law

**`LSIF_RECEIPT_ADMITTED` formula:**
```text
exit_code == 0
∧ lsif_file_exists
∧ blake3(lsif_file) == receipt.lsif_digest
∧ blake3(sorted canonical source_boundary files) == receipt.source_digest
∧ vertex_count > 0
∧ document_count > 0
∧ source_boundary ∩ {receipts/, target/, .git/} == ∅
```

Any failure → `status = STOP`, not `ADMITTED`.

**Self-reference guardrail:**
`source_boundary` must never include `receipts/`. Generating the receipt file must not mutate the hashed source tree.

**Count definitions (locked — do not improvise):**

| Field | Parser rule |
|---|---|
| `vertex_count` | LSIF lines where `"type":"vertex"` |
| `document_count` | vertices where `"label":"document"` |
| `moniker_count` | vertices where `"label":"moniker"` |
| `reference_count` | vertices where `"label":"referenceResult"` |

**Invariant: `STALE_LSIF_INDEX = STOP`**

TRUE: `source_digest == receipt.source_digest` ∧ `lsif_digest == receipt.lsif_digest`.
FALSE: source changed after LSIF receipt ∨ digest mismatch ∨ file missing.
COUNTERFACTUAL: Modify indexed source after receipt → `STALE_LSIF_INDEX` fires → `admission_allowed = false`.
WITNESS: `receipts/v26.6.28-lsif.receipt.json` + LSIF output file digest.
REPAIR: Rerun `lsp-max-lsif` indexer and regenerate receipt.

Stale LSIF is worse than missing LSIF:
```text
MissingLsif → "I do not know"
StaleLsif   → "I know" (while lying)
```
Therefore: `STALE_LSIF_INDEX = STOP`, not WARNING.

### Full storage layer table

| Layer | Store | Path |
|---|---|---|
| Live document text / AST | `texter` + `tree-sitter` | hot |
| Derived LSP computations | `Salsa` | hot |
| Concurrent diagnostic staging | `papaya` | hot |
| Process/server registry | `DashMap` | warm |
| Semantic law graph | `Oxigraph` | cold |
| Code intelligence graph | `LSIF` | cold/warm |
| Admission proof | receipt + BLAKE3 | admission path |
| Process evidence | OCEL | cold/warm |

### Two-shape law

The coding agent acts. The architect agent formalizes. The LSP interrupts. The receipt admits.

These are distinct shapes:

| Shape | Produces | Failure mode |
|---|---|---|
| Doctrine/architecture agent | law, invariants, boundaries | can become abstract |
| Coding/action agent | commits, patches, test results | can outrun the law |
| LSP-as-ANDON | push, block, repair | must be wired correctly |
| Receipt | proof object | can be faked if not bound |

The repository needs a law surface that governs all shapes, because none of them is final authority.

```
R_B ⊢ A = μ_B(O*_B)
```

The agent's action loop is useful.
The architectural loop is useful.
Neither is admission.
The receipt admits.

### Source pipeline (complete)

```text
Source
→ texter (text/range correctness)
→ tree-sitter (incremental parse)
→ Salsa (incremental recomputation)
→ LSIF (persisted code intelligence graph)
→ Oxigraph (semantic law join layer)
→ Invariants (ANDON evaluation)
→ ANDON (interruption/gate)
→ Receipt (admission proof)
→ OCEL (process memory)
```

The agent does not understand the repo. The repo provides an admitted semantic index.

