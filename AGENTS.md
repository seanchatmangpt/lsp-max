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
flush_coordinator — 100ms debounce mpsc; emits CompositorReceipt (prefixes_fingerprint) after each push
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

**Gap:** a diagnostic code from server A will trigger ANDON even if only server B declared
that prefix. The per-server isolation the formal model describes is not enforced at merge
time; it is only enforced at construction time (each server either uses its own list or
falls back to the static defaults).

**Status: PARTIAL** — the union is a superset of every individual Λ_CD^(D). The
implementation is more restrictive than the formal claim: no law violation escapes. The
formal claim of per-server isolation is overstated relative to what the code enforces.

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
examples/anti-llm-cheat-lsp
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

Reference implementations:

- `crates/lsp-max-cli/src/nouns/gate.rs` — `GateService::gate_file_path()` and `GateService::check()`
- `crates/lsp-max-compositor/src/gate_file.rs` — `GateFile::for_workspace()`

Both use the same FNV-1a constants (`offset_basis = 0xcbf29ce484222325`, `prime = 0x100000001b3`) and format the hash with `{hash:016x}`.

### Proposed mitigation (convention, not enforcement)

Subagent prompts should include a gate-check preamble as the first Bash action:

```bash
lsp-max-cli gate check || exit 1
```

This reads the gate file, exits 1 if ANDON is set, and blocks further shell actions in that subagent invocation. It mirrors what the PreToolUse hook does in the parent session.

Subagent prompt authors are responsible for including this preamble. There is no structural mechanism that forces it.

### What this gap does not affect

- The parent session's gate enforcement is unaffected.
- The compositor continues to write the gate file correctly.
- `lsp-max-cli gate check` remains the canonical single-syscall check for any caller.

### Admitted / Refused / OPEN

```text
PreToolUse hook enforcement in parent session:   ADMITTED
Gate file written by compositor:                 ADMITTED
lsp-max-cli gate check available to subagents:  ADMITTED
Structural enforcement in subagent sessions:     REFUSED — hook boundary is not crossable
Convention-based mitigation (prompt preamble):   CANDIDATE — not structurally enforced
Subagent gate propagation overall:               OPEN
```

Do not collapse OPEN into ADMITTED. The gap is present until structural enforcement exists.

---

## Current Framework Status — 2026-06-13

### ADMITTED
- Concurrent fanout: O(max RTT) dispatch, N=500 in <1ms
- Λ_CD eager gate write: ~400ns (was 100ms debounce window)
- L7 Speciation: per-server C_D routing via `prefixes_for_server()` (union; see gap note below)
- Channel capacity: 512 (zero signal loss at N=500)
- Dynamic quorum debounce: flush fires at quorum or 2×spread (≤30ms cap)
- daachorse ANDON prefix matching: O(|code|) classification, asymmetry eliminated
- Workspace test suite: all tests ADMITTED except known OPEN items listed below
- Clippy `-D warnings`: ADMITTED (zero warnings in workspace crates)

### CANDIDATE
- papaya::HashMap for DiagnosticBuffer (DashMap contention elimination)
- kanal channel for FlushCoordinator (lower send latency — kanal integrated but not benchmarked at N=500)
- simd-json for JSON-RPC framing (larger scope change; not yet prototyped)

### OPEN
- Subagent gate propagation: PreToolUse hooks do not cross Agent session boundaries (structural gap — see Subagent Gate Propagation section)
- dx-verify sibling repo violations: `wasm4pm` codebase has uncommitted changes (`tps-metrics/Cargo.toml`) — outside this workspace
- gc006 sealed-repo test (`test_gc006_authority_surface_lock`): BLOCKED — wasm4pm sibling has uncommitted changes; test is a known expected failure until sibling is clean
- L7 Speciation per-server isolation: `MergeContext` uses workspace-wide union of ANDON prefixes; per-server `HashMap<server_id, Vec<String>>` routing is CANDIDATE (see L7 Speciation Status section)

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
