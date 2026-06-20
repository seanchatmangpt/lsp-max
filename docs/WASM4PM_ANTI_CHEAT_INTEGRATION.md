# WASM4PM Integration for Anti-LLM Cheating Detection

**Status**: CANDIDATE (awaiting sibling repo access for full compilation)

## Overview

This document outlines the complete refactoring of `lsp-max-anti-cheat` to use **wasm4pm** for formal process mining-based cheat detection. The shift moves from pure pattern matching to hybrid detection: traditional observations + Oracle class inference (A8–A12).

## Architecture

### Layers

```
┌─ Layer 1: Observations (Pattern Matching) ────────────────┐
│  scan_file() → raw patterns (tower-lsp, victory language) │
│  scan_directory() → cross-file contracts, refgraph        │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─ Layer 2: OCEL Emission (Event Log) ──────────────────────┐
│  observations_to_ocel() → FileScanned, PatternMatched     │
│  Map observations to OCEL Event/Object graph              │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─ Layer 3: Validation (Formal Analysis) ───────────────────┐
│  wasm4pm_compat::ocel::validate() → disjoint universes,   │
│  mandatory E2O cardinality, temporal continuity           │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─ Layer 4: Oracle Inference (Anomaly Detection) ────────────┐
│  wasm4pm::oracles::infer() → A8–A12 patterns              │
│  - A8: Audit log tampering                                │
│  - A9: Temporal anomalies                                 │
│  - A10: Causal violations                                 │
│  - A11: Unknown state collapse                            │
│  - A12: Cyclic dependencies                               │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─ Layer 5: Diagnostics (Human-Readable) ───────────────────┐
│  AntiLlmDiagnostic with oracle_class + confidence         │
│  Use FRESH_NAME_PAIRS for Oracle class names              │
│  Blocking gate integration                                │
└─────────────────────────────────────────────────────────┘
```

## Data Flow

### 1. Observations (Existing Pattern Matching)

**Input**: File path
**Output**: Vec<Observation>
**Examples**:
- `construct: "tower-lsp", kind: "raw_text", line: 42`
- `construct: "done", kind: "victory_language", line: 87`
- `construct: "assert_contains_string", kind: "test_smell", line: 105`

### 2. OCEL Event Emission

**New Function**: `observations_to_ocel(obs: &[Observation]) -> OCEL`

Converts observations into formal event log:

```rust
// File Object
OCELObject::new("file_abc123", "File")
    .with_attribute(OCELEventAttribute::string("path", "/path/to/src/lib.rs"))

// Pattern Object
OCELObject::new("pattern_xyz789", "Pattern")
    .with_attribute(OCELEventAttribute::string("kind", "raw_text"))
    .with_attribute(OCELEventAttribute::string("construct", "tower-lsp"))
    .with_attribute(OCELEventAttribute::integer("line", 42))

// FileScanned Event
OCELEvent::new("ev_file_scanned_abc", "FileScanned")
    .with_relationship("file_abc123")
    .with_timestamp(Utc::now())

// PatternMatched Event
OCELEvent::new("ev_pattern_xyz", "PatternMatched")
    .with_relationship("pattern_xyz789")
    .with_relationship("file_abc123")
    .with_timestamp(Utc::now())
```

### 3. OCEL Validation

**Function**: `wasm4pm_compat::ocel::validate::validate(&ocel, &config) -> ValidationReport`

Enforces formal axioms from Object-Centric Event Data literature:
- **Disjoint Universes**: event_ids ∩ object_ids = ∅
- **Mandatory E2O**: ∀event, ∃ related objects
- **Temporal Continuity**: ∀event, timestamp is defined

**Rejection Scenarios**:
- `IntersectingEventAndObjectUniverses` → A8 forensic signal (log tampering)
- `EventWithNoObjects` → A10 causal violation (orphaned event)
- `EventMissingTimestamp` → A9 temporal anomaly (incomplete log)

### 4. Oracle Inference

**Function** (future): `wasm4pm::oracles::infer(&ocel, &traces) -> Vec<(OracleClass, f64, Vec<String>)>`

Detects the five adversary classes:

#### A8: Audit Log Tampering
**Symptoms**: Events added/removed without causality chain
**Detection**:
- Receipt event with no ancestor computation events
- Observation count < 3 (too sparse to be natural)
- Hash chain broken (digest mismatch)

**Diagnostic**:
```rust
AntiLlmDiagnostic {
    code: "WASM4PM-AUDIT-TAMPERING",
    oracle_class: Some("A8".to_string()),
    confidence: Some(0.95),
    message: "Receipt r001 has no supporting computation events".to_string(),
    ..
}
```

#### A9: Temporal Anomalies
**Symptoms**: Events out of causal order
**Detection**:
- StateTransition timestamp < preceding event
- Receipt timestamp < related computation
- Event sequence violates causality law

**Diagnostic**:
```rust
AntiLlmDiagnostic {
    code: "WASM4PM-TEMPORAL-ANOMALY",
    oracle_class: Some("A9".to_string()),
    confidence: Some(0.88),
    message: "Receipt created at 2026-06-17T14:30 but computation at 14:32".to_string(),
    ..
}
```

#### A10: Causal Violation
**Symptoms**: Broken dependency chains
**Detection**:
- Diagnostic emitted without triggering tool call
- Tool call result stored without prior read
- State transition unsupported by preceding events

**Diagnostic**:
```rust
AntiLlmDiagnostic {
    code: "WASM4PM-CAUSAL-VIOLATION",
    oracle_class: Some("A10".to_string()),
    confidence: Some(0.92),
    message: "Diagnostic ANTI-LLM-SURFACE-001 has no parent PatternMatched event".to_string(),
    ..
}
```

#### A11: Unknown State Collapse
**Symptoms**: Premature resolution without evidence
**Detection**:
- ConformanceVector: UNKNOWN → ADMITTED without bridge event
- Fitness score: null → 1.0 without measurement
- Status: CANDIDATE → BLOCKED without blocking event

**Diagnostic**:
```rust
AntiLlmDiagnostic {
    code: "WASM4PM-UNKNOWN-COLLAPSE",
    oracle_class: Some("A11".to_string()),
    confidence: Some(0.85),
    message: "Conformance admitted when prior state was unknown (no resolution event)".to_string(),
    ..
}
```

#### A12: Cyclic Dependency
**Symptoms**: Circular causality
**Detection**:
- Tool A calls Tool B calls Tool A
- Dependency chain loops back to source
- Object references form cycle

**Diagnostic**:
```rust
AntiLlmDiagnostic {
    code: "WASM4PM-CYCLIC-DEPENDENCY",
    oracle_class: Some("A12".to_string()),
    confidence: Some(0.99),
    message: "Tool dependency cycle: Edit → Bash → Edit".to_string(),
    ..
}
```

### 5. Diagnostic Integration

**Structure** (updated):
```rust
pub struct AntiLlmDiagnostic {
    pub code: String,                    // e.g., "ANTI-LLM-SURFACE-001"
    pub category: String,
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub forbidden_implication: String,
    pub blocking: bool,
    pub required_correction: String,
    pub required_next_proof: String,
    pub oracle_class: Option<String>,    // NEW: "A8", "A9", ..., "A12"
    pub confidence: Option<f64>,         // NEW: [0.0, 1.0]
}
```

**to_lsp() Output**:
```
Code: WASM4PM-AUDIT-TAMPERING
Severity: ERROR
Message: Receipt r001 has no supporting computation events
Oracle Class: A8
Confidence: 95.00%
```

## Implementation Roadmap

### Phase 1: OCEL Emission (DONE)
- ✅ Updated `Cargo.toml` to add wasm4pm dependency
- ✅ Refactored `diagnostics.rs` to include oracle_class + confidence
- ✅ Rewrote `engine.rs` with `observations_to_ocel()` function
- ✅ Exported new function in `lib.rs`

**Files Changed**:
- `crates/lsp-max-anti-cheat/Cargo.toml`
- `crates/lsp-max-anti-cheat/src/diagnostics.rs`
- `crates/lsp-max-anti-cheat/src/engine.rs`
- `crates/lsp-max-anti-cheat/src/lib.rs`

### Phase 2: OCEL Validation (NEXT)
- [ ] Add `validate_ocel()` function to engine
- [ ] Call `wasm4pm_compat::ocel::validate` in `evaluate_diagnostics_with_config`
- [ ] Map validation errors to `AntiLlmDiagnostic` with Oracle class
- [ ] Test with wasm4pm-compat available

### Phase 3: Oracle Inference (FUTURE)
- [ ] Add `infer_oracles()` function to engine
- [ ] Call `wasm4pm::oracles::infer` in `evaluate_diagnostics_with_config`
- [ ] Merge Oracle results with traditional diagnostics
- [ ] Use `FRESH_NAME_PAIRS` for human-readable names

### Phase 4: Gate Integration (FUTURE)
- [ ] Update `lsp-max-cli gate check` to call anti-cheat engine
- [ ] Report Oracle violations with confidence scores
- [ ] Block ANDON gate on A8–A12 violations
- [ ] Expose via `max/admissibility` diagnostic

### Phase 5: Negative Control (FUTURE)
- [ ] Run OCEL inference on clean sessions (CI-only)
- [ ] Collect baseline Oracle signatures
- [ ] Compare agent sessions against baseline
- [ ] Reduce false positives via statistical learning

## Dependency Tree

```
lsp-max-anti-cheat
├── wasm4pm-compat (existing)
│   ├── OCEL types
│   ├── validate function
│   └── FRESH_NAME_PAIRS
├── wasm4pm (new)
│   ├── Oracle algorithms (A8–A12)
│   ├── Conformance checking
│   └── Process discovery
└── [traditional parsers: tree-sitter, regex, etc.]
```

## Binary Size Trade-off

**User Directive**: "I do not care how big LSP binaries are. It is better to use combinatorial maximalism vs premature limitations."

**Result**:
- wasm4pm adds ~5–10 MB to binary (process mining algorithms + dependencies)
- Comprehensive coverage (500+ violation patterns) vs optimization
- No size-based feature gates or lazy loading
- Full Oracle inference available in all builds

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_observations_to_ocel() {
    let obs = vec![/* sample observations */];
    let ocel = observations_to_ocel(&obs);
    assert!(ocel.events.len() > 0);
    assert!(ocel.objects.len() > 0);
}

#[test]
fn test_oracle_a8_audit_tampering() {
    let ocel = /* create tampered log */;
    let report = wasm4pm_compat::ocel::validate::validate(&ocel, &config);
    assert!(!report.valid);
}
```

### Negative Controls
```bash
# Clean session (CI)
cargo test -p gc005-wasm4pm-adapter --test dogfood_*
# Extract baseline Oracle signatures

# Compromised session (agent)
# Apply wasm4pm::oracles::infer
# Compare against baseline
# Detect deviations
```

### Integration Tests
```bash
cargo test -p lsp-max-anti-cheat --test integration
cargo test --test e2e
```

## Future Enhancements

### 1. Temporal Profiling
- Extract session timeline from OCEL
- Detect anomalies: tool calls faster than disk I/O, etc.

### 2. Process Model Learning
- Build expected process model from clean sessions
- Detect conformance deviations automatically

### 3. Breed Conformance
- Validate breeds against wasm4pm process models
- Detect algorithm substitution (wrong breed injected)

### 4. Receipt Chain Forensics
- Rebuild receipt chain from OCEL events
- Verify cryptographic continuity

### 5. Multi-agent Orchestration
- Analyze coordination between multiple agents
- Detect race conditions, deadlock patterns

## References

- **OCEL Spec**: van der Aalst et al., Object-Centric Process Querying & Constraints
- **wasm4pm**: Sibling repo with Oracle algorithms and conformance engines
- **wasm4pm-compat**: Sibling repo with OCEL type definitions and FRESH_NAME_PAIRS
- **lsp-max-runtime**: Control plane with admission laws and graduation gates
