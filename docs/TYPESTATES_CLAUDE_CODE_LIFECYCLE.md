# Typestates for the Claude Code Lifecycle

A type-safe model of valid state transitions during a Claude Code session.

---

## Overview

A **typestate** is a Rust pattern that uses the type system to enforce valid state transitions. Each state is a distinct type, and only valid transitions are type-safe.

The Claude Code lifecycle consists of discrete phases, each with specific allowed actions and preconditions. Typestates guarantee that agents cannot violate these invariants at compile time.

---

## State Graph

```
    ┌─────────────────────────────────────────────────┐
    │ INITIAL                                         │
    │ - Session created                               │
    │ - Zero files modified                           │
    │ - No tool invocations logged                     │
    └──────────────────┬──────────────────────────────┘
                       │ initialize()
                       ↓
    ┌─────────────────────────────────────────────────┐
    │ READY                                           │
    │ - Codebase introspected                         │
    │ - Tools available                               │
    │ - No pending actions                            │
    └──────────────────┬──────────────────────────────┘
                       │ begin_task()
                       ↓
    ┌─────────────────────────────────────────────────┐
    │ EXECUTING                                       │
    │ - Task active                                   │
    │ - Tool invocations allowed                      │
    │ - Mutations pending (uncommitted)               │
    │ ┌─────────┬─────────┬──────────────────────┐   │
    │ │ (Sub-states)                             │   │
    │ │ - READING (Read tools active)            │   │
    │ │ - EDITING (Edit tools active)            │   │
    │ │ - EXECUTING (Bash/subprocess)            │   │
    │ │ - DECIDING (Awaiting user input)         │   │
    │ └─────────┴─────────┴──────────────────────┘   │
    └──────────────────┬──────────────────────────────┘
                       │ commit() or revert()
                       ↓
    ┌─────────────────────────────────────────────────┐
    │ CLOSING                                         │
    │ - All changes committed to git                  │
    │ - Receipt chain validated                       │
    │ - Diagnostics finalized                         │
    └──────────────────┬──────────────────────────────┘
                       │ finalize()
                       ↓
    ┌─────────────────────────────────────────────────┐
    │ CLOSED                                          │
    │ - Session complete                              │
    │ - All mutations admitted                        │
    │ - Audit trail sealed                            │
    └─────────────────────────────────────────────────┘
```

---

## State Definitions

### INITIAL
**Preconditions**:
- Session timestamp recorded
- Zero file mutations
- Zero tool invocations
- Codebase hash computed

**Allowed transitions**:
- → READY (via `initialize()`)

**Invariant violations detected in this state**:
- (None; no actions allowed)

**Example**:
```rust
let session = Session::new(/**/);
// State: Initial
session.initialize()?;
// State: Ready
```

---

### READY
**Preconditions**:
- Codebase introspected (file list, hashes)
- Tool manifest loaded
- Agent capabilities enumerated
- No pending mutations

**Allowed transitions**:
- → EXECUTING (via `begin_task()`)
- → INITIAL (via `reset()`)
- → CLOSED (via `abort()`)

**Invariant violations detected in this state**:
- (None; no mutations allowed)

**Example**:
```rust
session.begin_task("Implement feature X")?;
// State: Executing
```

---

### EXECUTING
**Preconditions**:
- Task active and logged
- Files may be read, edited, executed
- Mutations accumulated (not yet committed)
- Tool invocations tracked

**Allowed transitions**:
- → READY (via `revert()`)
- → CLOSING (via `commit()`)
- Sub-states: READING, EDITING, EXECUTING, DECIDING

**Invariant violations detected in this state**:

| Violation | Detection | Action |
|-----------|-----------|--------|
| File read not in transcript | Tool invocation logged but output not in session | WARNING: Invisible I/O |
| File edited outside LSP surface | Bash `sed`, direct `fs::write` | ERROR: Mutation outside protocol |
| Tool called but result ignored | Subprocess exit recorded; result not used | WARNING: Zombie tool |
| Victory language in plan/comment | "Fully solved", "guaranteed to work" | ERROR: Overclaim |
| Hardcoded metric passed as proof | Return value looks synthetic (e.g., `100` or `42`) | ERROR: Synthetic proof |
| Receipt without boundary markers | Missing `-----BEGIN RECEIPT-----` | ERROR: Fake receipt |
| Log output claimed as receipt | `stdout` from `println!` in test | ERROR: Test stdout ≠ receipt |

**Sub-states** (mutual exclusion enforced):

| Sub-state | Allowed Tools | Invariants |
|-----------|---------------|-----------|
| READING | Read, Glob, Grep, ToolSearch | Cannot write; output must be consumed |
| EDITING | Edit, Write (on already-read files) | Cannot execute; changes staged |
| EXECUTING | Bash, subprocess tools | Cannot read/write files directly; state transitions logged |
| DECIDING | None (agent polls user) | Cannot invoke tools; awaiting input |

**Example**:
```rust
session.enter_executing()?;  // State: Executing

// Sub-state: READING
session.read_file("src/main.rs")?;
session.grep("pattern", "*.rs")?;

// Sub-state: EDITING (transition automatic)
session.edit_file("src/main.rs", old, new)?;

// Sub-state: EXECUTING (transition automatic)
session.bash("cargo test")?;

// Back to READY
session.commit("Fix: implement feature X")?;
// State: Closing
```

---

### CLOSING
**Preconditions**:
- All mutations committed to git
- Receipt chain validated (SHA256 digests, boundaries)
- Diagnostics finalized (all violations resolved or accepted)
- No pending uncommitted changes

**Allowed transitions**:
- → CLOSED (via `finalize()`)
- → EXECUTING (via `amend()` or `reopen()`)

**Invariant violations detected in this state**:

| Violation | Detection | Action |
|-----------|-----------|--------|
| Uncommitted changes remain | `git status --porcelain` not empty | ERROR: Incomplete commit |
| Receipt chain broken | SHA256 digest mismatch in ledger | ERROR: Receipt tampered |
| Blocking diagnostics unresolved | `summary.blocking > 0` in final scan | ERROR: Admissibility gate failed |
| Version mismatch | Manifest version ≠ CalVer(date) | ERROR: Law violation |
| Tower-lsp references present | Final anti-llm-cheat scan | ERROR: Surface violation |
| Victory language in commit message | "Fully solved", "guaranteed" | ERROR: Overclaim |

**Example**:
```rust
session.commit("Final: Fix all violations")?;
// State: Closing

// Validation happens automatically
// - Git commit succeeds
// - Receipt chain verified
// - Anti-cheat scan runs
// - If all pass:
session.finalize()?;
// State: Closed

// If any violation:
// ERROR: Cannot transition; return to EXECUTING
```

---

### CLOSED
**Preconditions**:
- All mutations admitted (zero blocking diagnostics)
- Receipt chain sealed and cryptographically verified
- Audit trail immutable
- Session timestamp recorded

**Allowed transitions**:
- (None; session is finalized)

**Invariant violations detected in this state**:
- (None; session is read-only)

**Example**:
```rust
session.finalize()?;
// State: Closed
// All mutations admitted; session sealed
assert_eq!(session.phase(), Phase::Closed);
```

---

## Type-Safe Transitions

### Compile-Time Safety

Using Rust's type system, invalid transitions are **compile errors**:

```rust
// ✅ Valid: INITIAL → READY → EXECUTING
let session = Session::<Initial>::new();
let session = session.initialize()?;  // Session::<Ready>
let session = session.begin_task("task")?;  // Session::<Executing>

// ❌ Invalid: INITIAL → EXECUTING (compile error!)
let session = Session::<Initial>::new();
session.begin_task("task")?;  // ❌ Error: no method begin_task on Session::<Initial>

// ❌ Invalid: READY → EXECUTING → READY (compile error!)
let session = session.begin_task("task")?;  // Session::<Executing>
session.initialize()?;  // ❌ Error: no method initialize on Session::<Executing>
```

### State Encoding

```rust
// State markers (zero-cost abstractions)
pub struct Initial;
pub struct Ready;
pub struct Executing;
pub struct Closing;
pub struct Closed;

// Session parameterized by state
pub struct Session<State = Initial> {
    inner: SessionData,
    _state: PhantomData<State>,
}

// Transitions consume and return new type
impl Session<Initial> {
    pub fn initialize(self) -> Result<Session<Ready>> {
        // Perform initialization
        Ok(Session {
            inner: self.inner,
            _state: PhantomData,
        })
    }
}

impl Session<Ready> {
    pub fn begin_task(self, task: &str) -> Result<Session<Executing>> {
        // Start task
        Ok(Session {
            inner: self.inner,
            _state: PhantomData,
        })
    }
}
```

---

## Sub-State Machine (EXECUTING)

Within the EXECUTING state, tools have mutual-exclusion rules:

```
┌─────────────────────────────────────┐
│ EXECUTING (parent state)            │
├─────────────────────────────────────┤
│  ┌──────────┐                       │
│  │ READING  │ ← Entry point         │
│  │ (Read    │                       │
│  │  tools)  └─────────┐             │
│  └──────────────────────┬───────┐   │
│         │              │       │   │
│         │ (read done)  │       │   │
│         ↓              │       │   │
│  ┌──────────────┐      │       │   │
│  │ EDITING      │      │       │   │
│  │ (Edit/Write  │      │       │   │
│  │ tools)       │      │       │   │
│  └──────────────┘      │       │   │
│         │              │       │   │
│         │ (edit done)  │       │   │
│         ↓              │       │   │
│  ┌──────────────┐      │       │   │
│  │ EXECUTING    │      │       │   │
│  │ (Bash)       │      │       │   │
│  └──────────────┘      │       │   │
│         │              │       │   │
│         │ (ready to    │       │   │
│         │  commit)     │       │   │
│         └──────────────┴───────┘   │
│                │                   │
│         (can loop within           │
│          EXECUTING)                │
│                │                   │
│                ↓                   │
│         ┌─────────────┐            │
│         │ Commit or   │            │
│         │ Revert      │            │
│         └─────────────┘            │
└─────────────────────────────────────┘
```

**Invariant**: Only one sub-state active at a time.

**Transition rules**:
- READING → EDITING (automatic when first Edit tool called)
- EDITING → EXECUTING (automatic when first Bash tool called)
- EXECUTING → (loop back to READING) (when returning to Read tools)

---

## Invariant Enforcement

### Per-State Checks

Each state enforces specific invariants via the anti-llm-cheat system:

```rust
impl<State> Session<State> {
    fn check_invariants(&self) -> Result<Vec<AntiLlmDiagnostic>> {
        // State-specific checks
        match self.phase() {
            Phase::Initial => self.check_invariants_initial(),
            Phase::Ready => self.check_invariants_ready(),
            Phase::Executing => self.check_invariants_executing(),
            Phase::Closing => self.check_invariants_closing(),
            Phase::Closed => self.check_invariants_closed(),
        }
    }
    
    fn check_invariants_executing(&self) -> Result<Vec<AntiLlmDiagnostic>> {
        let mut violations = Vec::new();
        
        // Check all tool invocations are logged
        for tool_call in &self.tool_calls {
            if !self.transcript.contains(tool_call) {
                violations.push(AntiLlmDiagnostic {
                    code: "ANTI-LLM-ROUTE-001".to_string(),
                    message: "Tool invocation not visible in transcript".to_string(),
                    blocking: true,
                });
            }
        }
        
        // Check no victory language
        if self.transcript.contains("fully solved") {
            violations.push(AntiLlmDiagnostic {
                code: "ANTI-LLM-CLAIM-004".to_string(),
                message: "Victory language detected".to_string(),
                blocking: true,
            });
        }
        
        // Check receipts are valid
        for receipt in &self.receipts {
            if !receipt.has_valid_blake3() {
                violations.push(AntiLlmDiagnostic {
                    code: "ANTI-LLM-RECEIPT-003".to_string(),
                    message: "Missing or invalid cryptographic digest".to_string(),
                    blocking: true,
                });
            }
        }
        
        Ok(violations)
    }
}
```

### Transition Conditions

Transitions succeed **only if** all invariants for the target state are met:

```rust
impl Session<Executing> {
    pub fn commit(self, message: &str) -> Result<Session<Closing>> {
        // Enforce preconditions for CLOSING state
        
        // 1. Check no victory language in message
        if message.contains("solved") || message.contains("guaranteed") {
            return Err("Victory language in commit message".to_string());
        }
        
        // 2. Check all mutations are staged
        if !self.git_status_is_clean()? {
            return Err("Uncommitted changes remain".to_string());
        }
        
        // 3. Check receipt chain
        self.validate_receipt_chain()?;
        
        // 4. Run final anti-cheat scan
        let scan = run_anti_cheat_check(".")?;
        if scan.summary.blocking > 0 {
            return Err("Blocking violations prevent transition".to_string());
        }
        
        // 5. Git commit
        self.git.commit(message)?;
        
        // Now safe to transition
        Ok(Session {
            inner: self.inner,
            _state: PhantomData,
        })
    }
}
```

---

## Timeline / Event Log

Every transition is logged with cryptographic proof:

```
Session Timeline (OCEL-compatible):

[INITIAL → READY]
  timestamp: 2026-06-17T12:00:00Z
  action: initialize()
  codebase_hash: sha256:abc123...
  receipt: blake3:def456...

[READY → EXECUTING]
  timestamp: 2026-06-17T12:05:00Z
  action: begin_task("Implement feature X")
  receipt: blake3:ghi789...

[EXECUTING: READING]
  timestamp: 2026-06-17T12:05:30Z
  action: read_file("src/main.rs")
  receipt: blake3:jkl012...

[EXECUTING: EDITING]
  timestamp: 2026-06-17T12:06:00Z
  action: edit_file("src/main.rs", old, new)
  receipt: blake3:mno345...

[EXECUTING: EXECUTING]
  timestamp: 2026-06-17T12:06:30Z
  action: bash("cargo test")
  exit_code: 0
  receipt: blake3:pqr678...

[EXECUTING → CLOSING]
  timestamp: 2026-06-17T12:07:00Z
  action: commit("Fix: implement feature X")
  mutation_count: 1
  receipt: blake3:stu901...

[CLOSING → CLOSED]
  timestamp: 2026-06-17T12:07:30Z
  action: finalize()
  admission_status: ADMITTED
  receipt: blake3:vwx234...
  seal: RECEIPT_CHAIN_VALID
```

Every event has:
- Timestamp (UTC)
- Action (what happened)
- Proof (BLAKE3 receipt or exit code)
- Immutable hash linking to previous event

---

## Summary Table

| State | Phase | Allowed Actions | Invariants | Violations Checked |
|-------|-------|-----------------|-----------|-------------------|
| **INITIAL** | Setup | None (read-only) | Zero mutations | (None) |
| **READY** | Idle | Introspection | No pending mutations | (None) |
| **EXECUTING** | Active | Read, Edit, Execute | Tools logged, receipts valid | Victory language, fake receipts, invisible I/O |
| **CLOSING** | Finalizing | Commit/Revert | Changes committed | Uncommented changes, broken receipts, version law |
| **CLOSED** | Complete | None (read-only) | All admitted | (None; sealed) |

---

## Integration with Anti-Cheat

The typestate machine **guarantees**:
- ✅ No state leakage (type system enforces)
- ✅ No invalid transitions (compile-time checks)
- ✅ All transitions logged (immutable audit trail)
- ✅ Receipts cryptographic (BLAKE3 chain)

The anti-llm-cheat system **validates**:
- ✅ Invariants per state (runtime checks)
- ✅ Violations detected before transition (blocking)
- ✅ Violations recorded in audit trail (immutable)

Together: **Provably safe state machine.**

---

## Usage in Agents

Agents must respect the typestate machine:

```rust
// Agent code (type-safe by construction)
let session = Session::new();
let session = session.initialize()?;
let session = session.begin_task("task description")?;

// Can only call tools valid in EXECUTING state
session.read_file("src/main.rs")?;  // ✅
session.edit_file("src/main.rs", old, new)?;  // ✅
session.bash("cargo test")?;  // ✅

// Cannot call invalid operations
session.initialize()?;  // ❌ Compile error

// Transition to closing
session.commit("Fix: describe what was done")?;  // Must pass invariant checks
// If checks fail: returned to EXECUTING state for remediation
// If checks pass: transitioned to CLOSING

// Finalize
session.finalize()?;  // Returns CLOSED
// Session is now sealed; no further mutations allowed
```

---

## References

- **Typestate pattern**: https://docs.rust-embedded.org/book/static-guarantees/typestate.html
- **OCEL (Object-Centric Event Logs)**: For audit trail format
- **Anti-LLM Cheat Diagnostics**: `ANTI-LLM-ROUTE-*`, `ANTI-LLM-CLAIM-*`, `ANTI-LLM-RECEIPT-*`

---

**Status**: CANDIDATE (designed; awaiting implementation in runtime)
