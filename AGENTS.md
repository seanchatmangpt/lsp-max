# AGENTS.md SPR — tower-lsp-max

This SPR is the compressed activation layer for the full `AGENTS.md`. It does not replace the full file. It primes agents with the project laws before they read the detailed rules.

---

## Core Frame

`tower-lsp-max` is the proving ground for **inverted LSP**.

Normal LSP helps humans write code.

Inverted LSP makes the repository speak back to agents while they work.

The repository is not passive text. It is a law-bearing system.

`AGENTS.md` is the constitution.  
LSP is the live enforcement operator.  
`anti-llm-lsp` is the diagnostic canary.  
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

## Operating Metaphor

This project is an F1 race team for agents.

The agent is the driver.  
`tower-lsp-max` is the chassis/protocol surface.  
`anti-llm-lsp` is telemetry.  
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

## anti-llm-lsp

Build here:

```text
examples/anti-llm-lsp
```

Purpose:

```text
anti-llm-lsp runs on tower-lsp-max
anti-llm-lsp does not depend on plain tower-lsp
anti-llm-lsp exercises LSP 3.18 surfaces
anti-llm-lsp detects attempts to reintroduce tower-lsp
```

Self-sealing law:

```text
tower-lsp-max hosts anti-llm-lsp
anti-llm-lsp detects tower-lsp
therefore tower-lsp-max cannot silently regress to tower-lsp
```

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
```

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

## Final Prime

This project is not about making an LSP demo.

It is about making `AGENTS.md` enforceable during agent work.

`AGENTS.md` is law.  
Repo state is the manifold.  
Agent edits are trajectories.  
Failsets are curvature.  
Receipts are proof measure.  
LSP is the differential operator.  
Diagnostics are gradients.  
Code actions are constrained control vectors.  
LSP 3.18 is the enforcement basis.  
Admissibility is `Φ_G = 0` plus `R_B ⊢ A = μ(O*_B)`.  
Effective agent velocity is `dA_admitted/dt`.

Do not optimize for raw output.

Optimize for admitted work.
