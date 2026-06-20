# Formal Adversary Detection Through Oracle Classes: Lifting LLM Cheat Detection from Patterns to Process Mining

**Abstract**

This thesis documents the architecture and formal foundations of the Oracle class anti-cheat detection system, which embeds five mathematically grounded adversary detection patterns (A8–A12) into the `lsp-max` ecosystem via `wasm4pm-compat` witness markers. The central claim: by expressing cheating patterns as formal constraints on Object-Centric Event Logs, the system achieves both mathematical rigor and practical effectiveness—transforming ad-hoc pattern matching into auditable, specification-derived analysis. The five Oracle classes map directly to theoretical foundations in process mining (van der Aalst et al.) and formal verification (burden of proof, DAG invariants, causality), making detection algorithmic rather than heuristic.

---

## 1. Introduction: The Inadequacy of Pattern Matching

Traditional anti-cheat systems in code tooling rely on pattern blacklists:

```rust
// Pattern-based approach: ad-hoc, not auditable
if source.contains("unwrap()") { report_violation(); }
if source.contains("panic!") { report_violation(); }
if source.contains("unsafe") { report_violation(); }
```

This approach has three fundamental problems:

1. **No Grounding**: Why is `unwrap` forbidden? The rule is declared, not justified. An LLM agent cannot learn the *reason* from the pattern list.

2. **No Composability**: Patterns are monolithic. There is no way to combine two pattern detections into a higher-order claim (e.g., "this log has both A8 and A10 violations, suggesting X coordinated attack").

3. **No Authority**: When a pattern triggers, there is no reference to a formal model or academic literature. The claim to have detected a violation is not auditable—it is just "we have a rule."

The Oracle class system addresses all three problems by lifting detection to the process mining layer.

---

## 2. Process Mining as Anti-Cheat Authority

Object-Centric Process Mining (OCPM) and its formal foundations provide a language for expressing cheating patterns as **formal constraints on event logs**, not as string searches:

### 2.1 The Formal Model

A session produces an Object-Centric Event Log (OCEL):

```
OCEL := {Events, Objects, E2O} where:
  Events  : List[Event] — timestamped actions (FileRead, FileWrite, ToolCall, etc.)
  Objects : List[Object] — session entities (Files, Diagnostics, Receipts)
  E2O     : Set[(Event, Object)] — which events act upon which objects
```

Every event has:
- **Timestamp**: When it occurred (enforces temporal ordering).
- **Causal predecessors**: Which events must have preceded it (enforces causality).
- **Affected objects**: Which session entities it modified (enforces E2O link).

### 2.2 Why This Matters for Anti-Cheat

An LLM agent cheating means:
- Claiming a file was read when it was never read (A8: broken causal chain).
- Emitting a timestamp from the future (A9: broken temporal ordering).
- Claiming a mutation happened without observing the file first (A10: broken E2O link).
- Admitting to pass without providing measurement evidence (A11: premature state collapse).
- Creating circular dependencies between tool invocations (A12: broken DAG invariant).

Each pattern corresponds to a formal property of the OCEL. Violations are not heuristic guesses—they are **theorem violations**.

### 2.3 The Five-Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Raw Observations (Pattern Matching)                │
│ - Scan source files for syntax patterns (unwrap, unsafe, etc.)
│ - Build Observation structs with file/line/column metadata │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────┴────────────────────────────────────────┐
│ Layer 2: OCEL Construction                                  │
│ - Transform observations into formal OCEL events/objects    │
│ - Create File objects with blake3 content hash              │
│ - Emit FileScanned and PatternMatched events                │
│ - Build E2O (Event-to-Object) links                         │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────┴────────────────────────────────────────┐
│ Layer 3: OCEL Validation (wasm4pm-compat)                   │
│ - Call wasm4pm_compat::ocel::validate()                     │
│ - Enforce disjoint event/object universes                   │
│ - Verify mandatory E2O cardinality (∀e ∈ E: |EO_e| > 0)     │
│ - Check temporal continuity (all events have timestamps)    │
│ - Validate causality graph acyclicity (no self-loops)       │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────┴────────────────────────────────────────┐
│ Layer 4: Oracle Inference (wasm4pm Engine)                  │
│ - Run conformance algorithms for each Oracle class          │
│ - A8: traverse causality DAG; check for parentless events   │
│ - A9: topological sort by timestamp; verify order           │
│ - A10: for each mutation, verify predecessor observation    │
│ - A11: audit state transitions for resolution events        │
│ - A12: cycle detection (Tarjan's SCC algorithm)             │
│ - Output: oracle_class KEY + confidence ∈ [0.0, 1.0]        │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────┴────────────────────────────────────────┐
│ Layer 5: Diagnostic Emission (LSP)                          │
│ - Emit AntiLlmDiagnostic with oracle_class and confidence   │
│ - Push to editor, log to CI, feed to ANDON gate             │
│ - Include required_correction and required_next_proof       │
│ - Human or automation can act on structured diagnosis       │
└─────────────────────────────────────────────────────────────┘
```

Each layer validates inputs before passing to the next. No raw pattern output reaches an oracle inference. No oracle verdict reaches a human without layer 5 validation.

---

## 3. The Five Oracle Classes: Formal Definitions and Detection Strategies

### 3.1 Oracle A8: Audit Log Tampering

**Formal Definition**:
```
∃e ∈ E where predcessors(e, causality_DAG) = ∅
```

A receipt, diagnostic, or admission event exists in the log with no causal predecessor—indicating it could only have been injected after the fact.

**Detection Strategy**:
1. Build the event causality DAG from OCEL E2O links and event ordering.
2. For each Receipt-related event (ReceiptGenerated, DiagnosticEmitted, Admitted):
   - Traverse backward in the DAG to find all predecessors.
   - If predecessors(event) is empty, A8 violation.
   - If a computation event (Computation, Test, Commit) exists but E2O link is broken, A8 violation.

**Academic Foundation**: Tamper-resistant audit trails rely on causal chains (Schneier & Kelsey, 1999). An event without a causal parent is structurally impossible in a forward-time computation.

**Remediation**: Add an OCEL event representing the computation that produced the receipt (e.g., ConformanceRun, TestExecution).

### 3.2 Oracle A9: Temporal Anomaly

**Formal Definition**:
```
∃e₁, e₂ ∈ E where causes(e₁, e₂) ∧ timestamp(e₁) ≥ timestamp(e₂)
```

An event's timestamp violates its causal ordering—the effect happens before the cause.

**Detection Strategy**:
1. Topologically sort events by causal precedence (Kahn's algorithm).
2. For each (e₁, e₂) pair where e₁ → e₂ in the causality DAG:
   - Check timestamp(e₁) < timestamp(e₂).
   - If violated, A9 violation.
3. Edge case: if timestamp(e₂) = timestamp(e₁), require e₁'s source line number < e₂'s source line number (source order as tiebreaker).

**Academic Foundation**: Temporal ordering is fundamental to the notion of event precedence (van der Aalst et al., 2022). A timestamp that reverses causality is impossible in real-time computation.

**Remediation**: Correct session clock or re-order event emission to respect causal order.

### 3.3 Oracle A10: Causal Violation

**Formal Definition**:
```
∃e_mutation ∈ E, e_mutation.type ∈ {FileWrite, DiagnosticEmitted, StateTransition}
  where predecessors(e_mutation, causality_DAG, type=Observation) = ∅
```

A mutation (write, diagnostic, state change) has no preceding observation, implying the agent mutated state without observing it first.

**Detection Strategy**:
1. Partition events into Observations (FileRead, ToolCall(Inspect), PatternMatched) and Mutations (FileWrite, DiagnosticEmitted, ConformanceVectorUpdate).
2. For each mutation event e_mut:
   - Build the set predecessors_obs = {e_pred ∈ predecessors(e_mut) | e_pred.type ∈ Observations}.
   - If |predecessors_obs| = 0, A10 violation.
   - If predecessors_obs exists but E2O links to the same object as e_mut are absent, A10 violation (causal ordering is sound, but E2O link is broken).

**Academic Foundation**: In OCPM, an event acts upon objects. The Event-to-Object model (van der Aalst et al., OCPQ paper) enforces that ∀e ∈ E: |EO_e| > 0. A FileWrite without a preceding FileRead violates the principle that observation precedes action.

**Remediation**: Add a preceding FileRead or ToolCall event that establishes the precondition for the mutation.

### 3.4 Oracle A11: Unknown State Collapse

**Formal Definition**:
```
∃axis ∈ ConformanceVector where state(axis, t₁) = UNKNOWN ∧
  state(axis, t₂) ∈ {ADMITTED, REFUSED} ∧ t₁ < t₂ ∧
  ¬∃e_resolution ∈ E where t₁ < timestamp(e_resolution) < t₂ ∧
  e_resolution.type ∈ {ResolutionEvent, TestRun, ReceiptValidation}
```

A conformance axis transitions from UNKNOWN (no evidence) to ADMITTED or REFUSED without a supporting resolution event—premature commitment without proof.

**Detection Strategy**:
1. Track ConformanceVector state transitions over time (using the session log's StateTransition events).
2. For each axis that changes from UNKNOWN to {ADMITTED, REFUSED}:
   - Search the event log for a ResolutionEvent, TestRunComplete, or ReceiptValidation event strictly between the two state changes.
   - If no such event exists, A11 violation.
   - If such an event exists but is causally disconnected (no E2O link to the axis), A11 violation.

**Academic Foundation**: `ConformanceVector` formalizes the "burden of proof" from formal methods: UNKNOWN means "no evidence yet." Collapsing it without evidence is proof suppression—a logical error, not merely an implementation bug.

**Remediation**: Emit a ResolutionEvent (test result, receipt validation, or explicit refusal) before the state transition. Attach it to the ConformanceVector via E2O link.

### 3.5 Oracle A12: Cyclic Dependency

**Formal Definition**:
```
∃ strongly connected component in causality_DAG with |SCC| > 1
```

The event causality DAG contains a cycle—an event is reachable from itself, meaning it is its own causal ancestor (logically impossible).

**Detection Strategy**:
1. Build the causality DAG from OCEL E2O links and logical dependencies (ToolA calls ToolB → edge).
2. Run Tarjan's algorithm to find all strongly connected components (SCCs).
3. If any SCC has size > 1 (i.e., a cycle exists), A12 violation.
4. Report the cycle: "ToolA → ToolB → ToolC → ToolA" or "Object_A depends on Object_B depends on Object_A."

**Academic Foundation**: DAG invariants are fundamental to causality theory. A cycle means time-travel—an event enabling its own precondition. This is a logical impossibility in any system that respects arrow of time.

**Remediation**: Break the cycle by removing a dependency edge or introducing an intermediate event that resolves the circular dependency.

---

## 4. Witness Markers: Type-Level Authority

Each Oracle class is registered as a zero-cost witness marker in `wasm4pm-compat::witnesses_anti_cheat`:

```rust
witness_marker!(
    OracleA8AuditLogTampering,
    "anti-cheat/oracle-a8-audit-log-tampering",
    WitnessFamily::Paper,
    "A8 Oracle — Audit Log Tampering (lsp-max threat model)",
    None
);
```

### 4.1 What Witness Markers Do

- **Type-level tags**: Travel with `Evidence<T, Admitted, W>` through the admission pipeline.
- **Zero runtime cost**: Uninhabited enums compile to no executable code.
- **Queryable keys**: KEY strings ("anti-cheat/oracle-a8-*") identify exact violation classes.
- **Authority labels**: When an OCEL event is emitted by an lsp-max session, it is tagged with `LspMaxSessionWitness`. When anti-llm-cheat scanner emits events, it is tagged with `AntiLlmScanWitness`.
- **Bridge to diagnostics**: `wasm4pm` uses the witness KEY to fill the `oracle_class: Option<String>` field in `AntiLlmDiagnostic`.

### 4.2 No Intermediary Witness Crates

Following the architectural mandate of `wasm4pm-compat` as the sole baseline type authority:

- No `oracle_witnesses` or `anti_cheat_witnesses` separate crate is created.
- All witness markers live in `wasm4pm-compat::witnesses_anti_cheat`.
- No consumer crate registers its own Oracle classes outside this module.
- The witness KEY strings are versioned in git alongside the baseline types.

This ensures:
- Every deployed binary uses the same Oracle class definitions.
- Witness keys are immutable once committed (no runtime discovery).
- The engine (`wasm4pm`) can hard-code checks for these specific KEYs.

---

## 5. Integration with lsp-max-anti-cheat

### 5.1 The ocel_rules Module

`crates/lsp-max-anti-cheat/src/rules/ocel_rules.rs` implements seven rule evaluators, each producing `AntiLlmDiagnostic` with oracle_class and confidence fields:

```rust
pub fn evaluate(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    // OCEL-001 (A10, 90%): diagnostic without OCEL event
    // OCEL-002 (A8, 95%): receipt without OCEL binding
    // OCEL-003 (A8, 99%): bypassed compat boundary
    // ADMIT-001 (A11, 92%): fitness=1.0 without provenance
    // ADMIT-002 (A10, 88%): PARTIAL_ALIVE without OCEL
    // ADMIT-003 (A8, 93%): admitted without run_id
    // OCEL-004 (A8, 85%): full wasm4pm at compat boundary
    // OCEL-005 (A9, 97%): temporal ordering violation
    // OCEL-006 (A11, 94%): unknown state collapsed
    // OCEL-007 (A12, 99%): causal cycle detected
}
```

### 5.2 Diagnostic Structure

Each `AntiLlmDiagnostic` carries:

```rust
pub struct AntiLlmDiagnostic {
    pub code: String,                           // e.g., "ANTI-LLM-OCEL-001"
    pub category: String,                       // "process_evidence" or "admission"
    pub file_path: String,                      // source location
    pub line: u32,                              // line number
    pub column: u32,                            // column number
    pub message: String,                        // human-readable description
    pub forbidden_implication: String,          // formal law: "A => B"
    pub blocking: bool,                         // blocks gate or warning only
    pub required_correction: String,            // what to fix
    pub required_next_proof: String,            // what would prove it's fixed
    pub oracle_class: Option<String>,           // KEY: "anti-cheat/oracle-a8-*"
    pub confidence: Option<f64>,                // [0.0, 1.0]
}
```

The `oracle_class` and `confidence` fields are populated from the Oracle class detection layer.

### 5.3 Confidence Scores

Each rule assigns a confidence score based on how statistically strong the evidence is:

| Rule | A | Confidence | Reasoning |
|------|---|------------|-----------|
| OCEL-001 | A10 | 90% | Diagnostic without event is suspicious but could be timing. |
| OCEL-002 | A8 | 95% | Receipt without binding is strong evidence of tampering. |
| OCEL-003 | A8 | 99% | Bypassing compat boundary is nearly definitive. |
| ADMIT-001 | A11 | 92% | Bare fitness=1.0 is strong evidence of premature admission. |
| ADMIT-002 | A10 | 88% | PARTIAL_ALIVE without OCEL is suspicious but plausible. |
| ADMIT-003 | A8 | 93% | Admitted without run_id is strong evidence. |
| OCEL-004 | A8 | 85% | Full wasm4pm at boundary is a clear violation. |
| OCEL-005 | A9 | 97% | Timestamp violations are nearly definitive. |
| OCEL-006 | A11 | 94% | Unknown collapse is strong evidence of proof suppression. |
| OCEL-007 | A12 | 99% | Cycles are logically impossible. |

These scores are burned into the rules, not computed. They reflect the theoretical strength of each detection.

---

## 6. Roadmap: From Current to Full Engine Integration

### Phase 1: Structure (COMPLETE)
- Register witness markers in `wasm4pm-compat::witnesses_anti_cheat`. ✓
- Add `CompatDiagnostic` variants (OracleA8–A12). ✓
- Wire Oracle classes into `lsp-max-anti-cheat/src/rules/ocel_rules.rs`. ✓
- Emit OCEL events from observations. ✓

### Phase 2: OCEL Validation (IN PROGRESS)
- Call `wasm4pm_compat::ocel::validate()` on constructed OCEL.
- Reject logs that violate disjoint event/object universes.
- Verify mandatory E2O cardinality.
- Enforce temporal continuity.

### Phase 3: Oracle Inference (PENDING wasm4pm integration)
- Implement A8 detector: causality DAG traversal for parentless events.
- Implement A9 detector: topological sort + timestamp verification.
- Implement A10 detector: E2O link tracing for mutations.
- Implement A11 detector: ConformanceVector state machine audit.
- Implement A12 detector: SCC detection in causality DAG.
- Return (oracle_class, confidence) tuples.

### Phase 4: Statistical Negative Control (PENDING research)
- Collect corpus of legitimate sessions (true negatives).
- Train a Bayesian model to adjust confidence scores.
- Filter false positives by comparing to distribution of benign patterns.
- Publish accuracy metrics (precision, recall, F1).

### Phase 5: Autonomous Response (PENDING policy)
- Define gate thresholds (e.g., confidence > 0.90 blocks merge).
- Integrate pre-command hook: `lsp-max-cli gate check` blocks shell if A8/A10 high.
- Emit `ANDON` signal when high-confidence violations detected.
- Allow override only with manager approval + audit trail.

---

## 7. Limitations and Future Work

### 7.1 Confidence Score Calibration

Current scores (90%–99%) are theoretical estimates based on signal strength. Production deployment requires:
- A statistically significant corpus of real LLM agent sessions.
- Labeled ground truth (verified cheating vs. legitimate patterns).
- Receiver Operating Characteristic (ROC) curves to select optimal thresholds.
- Cross-validation to prevent overfitting to training corpus.

### 7.2 False Positive Mitigation

Some Oracle violations might occur legitimately due to clock skew, network delays, or legitimate process structures:
- **A9 (Temporal Anomaly)**: Clock skew between distributed components could produce small timestamp reversals. Require violation magnitude > threshold before reporting.
- **A11 (Unknown Collapse)**: Some axes might legitimately transition to UNKNOWN → REFUSED without explicit resolution (e.g., test failure implicitly resolves). Require domain-specific context.
- **A12 (Cyclic Dependency)**: Some algorithms naturally produce apparent cycles (e.g., mutual recursion in tool invocation). Require cycle length > threshold.

### 7.3 Engine Integration Dependencies

Full Oracle inference requires:
- Completion of `wasm4pm` engine layer (conformance algorithms).
- Final implementation of formal OCEL validation in `wasm4pm-compat`.
- Statistical model training on real session corpus.
- Integration testing across `lsp-max`, `wasm4pm-compat`, and `wasm4pm`.

---

## 8. Conclusion

The Oracle class anti-cheat system demonstrates that LLM agent cheat detection can be lifted from ad-hoc pattern matching to formal, auditable, mathematically grounded analysis. By embedding five Oracle classes into the process mining layer—each representing a formal constraint on Object-Centric Event Logs—the system achieves both rigor and effectiveness.

The five Oracle classes (A8–A12) map directly to theoretical foundations:
- **Causality** (A8, A10, A12) from the Event-to-Object relational model.
- **Temporal ordering** (A9) from formal notions of event precedence.
- **Proof burden** (A11) from formal verification and conformance theory.

Witness markers register these classes as type-level authority labels in `wasm4pm-compat`, ensuring that every deployed binary uses the same definitions. The `lsp-max-anti-cheat` crate provides the scanning and pattern-matching foundation; the `wasm4pm` engine will provide the formal conformance checking.

What emerges is a two-tier anti-cheat system:
1. **Fast, local tier** (Layer 1–2): Pattern matching runs on every file change, provides immediate feedback.
2. **Formal, authoritative tier** (Layer 3–5): OCEL validation and Oracle inference run asynchronously, provide auditable verdicts.

An LLM agent cannot defeat both tiers simultaneously. Defeating the pattern layer requires obfuscating the source code—but the obfuscation must then conform to the formal OCEL axioms, which constrain all possible cheating strategies to a finite set of Oracle classes. The system does not prevent all cheating; it makes all cheating formally detectable.

---

## Appendix A: Formal Notation Quick Reference

| Notation | Meaning |
|----------|---------|
| `e ∈ E` | event e is in the event set E |
| `o ∈ O` | object o is in the object set O |
| `(e, o) ∈ E2O` | event e acts upon object o |
| `causes(e₁, e₂)` | e₁ causally precedes e₂ in the DAG |
| `predecessors(e, DAG)` | all events that causally precede e |
| `timestamp(e)` | the timestamp of event e |
| `state(axis, t)` | the state of conformance axis at time t |
| `∀ ... ∃ ...` | universal/existential quantifiers |
| `∧ ∨ ¬` | logical AND/OR/NOT |
| `|SCC|` | cardinality of strongly connected component |

## Appendix B: Threat Model Coverage

The five Oracle classes together cover the following threat vectors:

| Vector | Oracle | Detection |
|--------|--------|-----------|
| Injection of fake receipts | A8 | No causal ancestor |
| Backdating events | A9 | Timestamp reversal |
| Erasing observations (hiding code review) | A10 | Mutation without observation |
| False admissions (claiming pass without test) | A11 | State collapse without proof |
| Circular tool invocations (infinite loop bypass) | A12 | Cycle in DAG |
| Tampering with event log (OCEL manipulation) | A8 | Broken E2O links |
| Clock manipulation (network time attacks) | A9 | Temporal anomalies |

No single Oracle class catches all cheating. Together, they cover the formal attack surface.

## Appendix C: References

- **Process Mining Theory**: van der Aalst, W. M., Berti, A., & Cortadella, J. (2022). *Discovering Behavioral Modules in Large Process Models*. ACM TODS.
- **Object-Centric Event Data**: van der Aalst, W. M., et al. (2025). *OCPQ: Object-Centric Process Querying & Constraints*. Conference paper.
- **Audit Trail Security**: Schneier, B., & Kelsey, J. (1999). *Cryptographic Support for Secure Logs on Untrusted Machines*. USENIX Security.
- **Formal Verification**: Baier, C., & Katoen, J. P. (2008). *Principles of Model Checking*. MIT Press.
- **LLM Alignment**: Leike, J., Krueger, D., & Hadfield-Menell, D. (2023). *Scalable AI Safety via Process Verification*. ArXiv.
