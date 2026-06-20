# Combinatorial Maximalism: All LLM Cheating Patterns

Complete enumeration of **every possible way** an LLM agent can violate law-state semantics.

---

## Introduction

**Combinatorial maximalism** means: rather than fixing violations as they're discovered (reactive), we enumerate **every possible violation axis** and design rules preventively (proactive).

This document lists 7 orthogonal **violation dimensions**, each with specific **violation modes**, giving ~500+ distinct cheating patterns. Each pattern has a corresponding diagnostic code in the anti-llm-cheat system.

---

## Violation Dimensions (Orthogonal Axes)

```
1. SURFACE (LSP library, capabilities)
2. AUTHORITY (Command, routing, abstraction)
3. RECEIPT (Proof, admission, evidence)
4. ROUTE (Execution proof, pathway, visibility)
5. MUTATION (File changes, side effects, purity)
6. VERSION (Temporal semantics, identity)
7. DETERMINISM (Reproducibility, oracles, metrics)
```

Each dimension has **modes** (specific violation types within that dimension).

---

## Dimension 1: SURFACE (LSP Library & Capabilities)

**Axis**: Which LSP library/framework is used? What capabilities are claimed?

### Mode 1.1: Library Identity Violation

**Violates**: "Codebase uses lsp-max, not tower-lsp"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Plain `tower_lsp` import | Text match: `use tower_lsp` | SURFACE-001 | Yes |
| Plain `tower_lsp::` namespace | Text match: `tower_lsp::*` | SURFACE-001 | Yes |
| `tower-lsp` Cargo dependency | TOML parse: `tower-lsp = *` | SURFACE-001 | Yes |
| Tower-lsp in Cargo.lock | Lock file scan | SURFACE-001 | Yes |
| tower-lsp reference in docs | Markdown scan | SURFACE-001 | No (warnings in legacy docs) |
| tower-lsp in comments | AST comment scan | SURFACE-001 | No |
| Indirect tower-lsp (via transitive dep) | Cargo tree analysis | SURFACE-001 | Yes |

**Severity**: BLOCKING (breaks LSP 3.18 compliance)

---

### Mode 1.2: Capability Negotiation Violation

**Violates**: "LSP 3.18 capabilities must be negotiated in initialize handshake"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Initialize without capabilities field | JSON-RPC parse: missing `capabilities` | SURFACE-005 | Yes |
| Hardcoded capabilities (not from config) | AST: literal dict instead of config | SURFACE-005 | Yes |
| Capabilities claim unimplemented features | LSP 3.18 matrix mismatch | SURFACE-005 | Yes |
| No initialize handshake at all | Transcript: client never sends initialize | SURFACE-005 | Yes |
| Initialize response missing required fields | JSON schema validation | SURFACE-005 | Yes |

**Severity**: BLOCKING (breaks LSP spec)

---

### Mode 1.3: Observer Pattern Violation

**Violates**: "LSP servers observe; they don't mutate files"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Direct file write via `std::fs::write` | AST: `fs::write`, `fs::File::create` | SURFACE-003 | Yes |
| Direct file write via `tokio::fs::write` | AST: `tokio::fs::write` | SURFACE-003 | Yes |
| File mutation via Bash subprocess | Bash: `echo > file`, `sed -i file` | SURFACE-003 | Yes |
| WorkspaceEdit used to mutate (not propose) | LSP parse: client should apply edits, not server | SURFACE-003 | Yes |
| Modifying files in workspace root | File monitor: unexpected changes | SURFACE-003 | Yes |

**Severity**: BLOCKING (violates read-only principle)

---

## Dimension 2: AUTHORITY (Commands, Routing, Abstraction)

**Axis**: Is command authority clear, explicit, and traced?

### Mode 2.1: CLAP Authority Violation

**Violates**: "Commands must use explicit `clap-noun-verb` CLI, not fake abstractions"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Fake CLAP class/struct (not actual clap) | Text search: `class CLAP`, `struct CLAP` | AUTH-002 | Yes |
| String-shaped command dispatch | AST: `if arg == "name"` instead of clap | AUTH-002 | Yes |
| Manual argv parsing (no clap) | AST: direct access to `std::env::args` | AUTH-002 | Yes |
| Regex-based command routing | AST: command routing via regex patterns | AUTH-002 | Yes |
| Hash-based command lookup | AST: `HashMap::from_iter` for commands | AUTH-002 | Yes |

**Severity**: BLOCKING (breaks authority tracing)

---

### Mode 2.2: String Authority Violation

**Violates**: "String content cannot be used as law or proof"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Law based on string content | Code: `if content.contains("assert"){ pass_rule() }` | STRANGE-007 | Yes |
| String regex as validator | Code: `Regex::new("pattern").is_match(s)` without bounds | STRANGE-007 | Yes |
| Display string as proof of type | Code: `format!("{:?}").contains("Field")` | STRANGE-007 | Yes |
| String comparison as equality | Code: `to_string() == to_string()` instead of `==` | STRANGE-007 | Yes |

**Severity**: BLOCKING (strings are not law)

---

### Mode 2.3: Routing Authority Violation

**Violates**: "Routing decisions must be logged and traceable"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Silent routing (no log) | Audit: RPC dispatch not logged | AUTH-004 | Yes |
| Routing decision in nested scope | AST: routing buried in conditionals | AUTH-004 | Yes |
| Dynamic route injection | AST: routes loaded from user input | AUTH-004 | Yes |
| Route handler swapping at runtime | AST: handlers reassigned | AUTH-004 | Yes |

**Severity**: BLOCKING (breaks auditability)

---

## Dimension 3: RECEIPT (Proof, Admission, Evidence)

**Axis**: Is every claim backed by cryptographic proof?

### Mode 3.1: Fake Receipt

**Violates**: "Receipts must have BLAKE3 digests, boundary markers, and ledger entries"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Test stdout claimed as receipt | Text: `println!("PASS")` in test | RECEIPT-001 | Yes |
| Log output claimed as receipt | Text: `eprintln!` or `log::info!` | RECEIPT-002 | Yes |
| Receipt without `-----BEGIN RECEIPT-----` | Format: missing boundary markers | RECEIPT-001 | Yes |
| Receipt without BLAKE3 digest | JSON: missing `blake3` field | RECEIPT-003 | Yes |
| Invalid BLAKE3 (wrong length) | Crypto: digest not 64 hex chars | RECEIPT-003 | Yes |
| BLAKE3 hash mismatch | Crypto: hash(content) ≠ stored digest | RECEIPT-003 | Yes |
| Receipt not in ledger | Audit: receipt not linked in chain | RECEIPT-001 | Yes |
| Duplicate receipts (same digest) | Audit: receipt appears twice | RECEIPT-001 | Yes |
| Receipt from future (timestamp > now) | Temporal: receipt dated after finalization | RECEIPT-001 | Yes |
| Receipt without checkpoint | Format: missing checkpoint field | RECEIPT-001 | Yes |

**Severity**: BLOCKING (breaks admission layer)

---

### Mode 3.2: Missing Evidence

**Violates**: "Every claim must have backing receipt"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Claimed feature has no transcript | LSP 3.18 matrix: code column empty | RECEIPT-002 | Yes |
| Claimed feature has no receipt | LSP 3.18 matrix: receipt column empty | RECEIPT-002 | Yes |
| Test passes without receipt | Test: `assert!(true)` with no proof | RECEIPT-001 | Yes |
| Measurement without baseline | Metric: "60% faster" with no before/after | RECEIPT-002 | Yes |
| Performance claim without benchmark | Perf: "N seconds" without reproducible benchmark | RECEIPT-002 | Yes |

**Severity**: BLOCKING (breaks evidence layer)

---

### Mode 3.3: Ledger Integrity Violation

**Violates**: "Receipt chain must be immutable and cryptographically linked"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Ledger reordered (receipts out of order) | Audit: timestamp sequence breaks | RECEIPT-002 | Yes |
| Ledger entry missing | Audit: gap in sequence numbers | RECEIPT-002 | Yes |
| Ledger hash chain broken | Crypto: receipt[i].prev_hash ≠ hash(receipt[i-1]) | RECEIPT-002 | Yes |
| Ledger modified after seal | Temporal: modification timestamp > seal timestamp | RECEIPT-002 | Yes |
| Ledger signed by wrong key | Crypto: signature invalid for key | RECEIPT-002 | Yes |
| Ledger timestamp not monotonic | Temporal: receipt[i].time > receipt[i+1].time | RECEIPT-002 | Yes |

**Severity**: BLOCKING (breaks audit trail immutability)

---

## Dimension 4: ROUTE (Execution Proof, Pathway, Visibility)

**Axis**: Can the execution pathway be proven and traced?

### Mode 4.1: Log-as-Route Confusion

**Violates**: "Log output is not route proof; route proof must be receipt"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Log output used as execution proof | Code: `println!("Route: X")` as proof | ROUTE-001 | Yes |
| Stderr message used as route proof | Code: `eprintln!("Executed")` | ROUTE-001 | Yes |
| Test output string as route proof | Test: `assert_contains!("executed")` | ROUTE-001 | Yes |
| Backtrace string as route proof | AST: backtrace parsing as control flow proof | ROUTE-001 | Yes |

**Severity**: BLOCKING (logs are not proofs)

---

### Mode 4.2: Static Analysis as Route Proof

**Violates**: "Code reading is not route proof; only execution receipts count"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| "Code path exists, therefore executed" | Logic: static analysis used as proof | ROUTE-008 | Yes |
| "Function defined, therefore called" | Logic: AST scan used as call proof | ROUTE-008 | Yes |
| "Tests exist, therefore pass" | Logic: file listing used as pass proof | ROUTE-008 | Yes |
| CFG analysis as execution proof | Logic: control flow analysis instead of trace | ROUTE-008 | Yes |

**Severity**: BLOCKING (execution requires actual proof)

---

### Mode 4.3: Invisible I/O

**Violates**: "All I/O must be visible in transcript"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Tool invoked but output not captured | Audit: Bash command logged, stdout/stderr missing | ROUTE-001 | Yes |
| File read not in transcript | Audit: file_hash computed, read operation not logged | ROUTE-001 | Yes |
| Network call not logged | Audit: HTTP request not in transcript | ROUTE-001 | Yes |
| Subprocess output discarded | Code: `.output()` called but result not used | ROUTE-001 | Yes |
| File written but not committed | Audit: `fs::write` happens, file not in git | ROUTE-001 | Yes |

**Severity**: BLOCKING (breaks observability)

---

## Dimension 5: MUTATION (File Changes, Side Effects, Purity)

**Axis**: Are mutations controlled, traceable, and admitted?

### Mode 5.1: Direct Mutation (Outside LSP)

**Violates**: "All mutations must be via LSP protocol, never direct fs access"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Direct `std::fs::write` | AST: `fs::write`, `File::create`, `OpenOptions` | MUT-001 | Yes |
| Direct `tokio::fs::write` | AST: async file ops | MUT-001 | Yes |
| Bash `echo > file` | Bash: output redirection | MUT-001 | Yes |
| Bash `sed -i` | Bash: in-place file editing | MUT-001 | Yes |
| Bash `cp` into source tree | Bash: file copy into src/ | MUT-001 | Yes |
| `git apply` in subprocess | Bash: patch application | MUT-001 | Yes |
| Symlink creation | AST/Bash: `symlink()` or `ln -s` | MUT-001 | Yes |
| File permission changes | Bash: `chmod` | MUT-001 | Yes |
| Binary file modification | Bash: `xxd`, hex editor | MUT-001 | Yes |

**Severity**: BLOCKING (breaks protocol purity)

---

### Mode 5.2: WorkspaceEdit Misuse

**Violates**: "WorkspaceEdit must be proposal to client, not assertion by server"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Server applies WorkspaceEdit to own workspace | Code: server calls `applyEdit` | MUT-002 | Yes |
| WorkspaceEdit without client ack | Protocol: edit sent, no `workspace/applyEdit` response | MUT-002 | Yes |
| Forced edit (ignoring client rejection) | Protocol: client rejects, server retries | MUT-002 | Yes |
| Batch mutations in single WorkspaceEdit | Protocol: multiple files in one request | MUT-002 | Yes |

**Severity**: BLOCKING (violates LSP semantics)

---

### Mode 5.3: Side Effects

**Violates**: "Functions must be pure (no hidden side effects)"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Function modifies global state | AST: static mut or thread-local mutation | MUT-002 | Yes |
| Function modifies input parameters | AST: parameter mutation without &mut | MUT-002 | Yes |
| Logging library with side effects | Code: `println!` in pure function | STRANGE-002 | No (warning) |
| Implicit database commit | Code: `db.commit()` not in signature | MUT-002 | Yes |
| Cache poisoning | Code: cache invalidation not explicit | MUT-002 | Yes |

**Severity**: BLOCKING (breaks functional semantics)

---

## Dimension 6: VERSION (Temporal Semantics, Identity)

**Axis**: Is temporal/version law respected?

### Mode 6.1: CalVer Violation

**Violates**: "Version must be CalVer (YY.M.D), not SemVer (X.Y.Z)"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Version is `1.0.0` (SemVer default) | TOML: `version = "1.0.0"` | VERSION-001 | Yes |
| Version is `0.1.0` (SemVer default) | TOML: `version = "0.1.0"` | VERSION-001 | Yes |
| Version does not match date | CalVer: version `26.6.9` on date 2026-06-18 | VERSION-001 | Yes |
| Version in past (before project start) | CalVer: version `24.1.1` but project is from 2026 | VERSION-001 | Yes |
| Version in future | CalVer: version `27.1.1` on date 2026-06-17 | VERSION-001 | Yes |
| Version has SemVer patch (3 parts = SemVer) | CalVer: `26.6.9.1` (4 parts required for CalVer) | VERSION-001 | Yes |
| Path dependency with explicit version | Cargo: `{ path = "...", version = "1.0.0" }` | VERSION-002 | Yes |
| Workspace version mismatch | Workspace: members have different versions | VERSION-003 | Yes |
| Version hardcoded in code | Code: `const VERSION: &str = "1.0.0"` | VERSION-001 | No (warning) |

**Severity**: BLOCKING (breaks CalVer law)

---

### Mode 6.2: Temporal Anomalies

**Violates**: "Timeline must be monotonic and causally coherent"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Commit timestamp before previous commit | Git: commit[i].time < commit[i-1].time | TRACE-001 | Yes |
| Receipt timestamp in future | Crypto: receipt.time > now() | RECEIPT-001 | Yes |
| Session duration > possible (e.g., 1 second scan of GB repo) | Audit: duration seems impossible | CHEAT-002 | Yes |
| Causality violation (effect before cause) | Logic: file modified before read | TRACE-001 | Yes |

**Severity**: BLOCKING (breaks causality)

---

## Dimension 7: DETERMINISM (Reproducibility, Oracles, Metrics)

**Axis**: Are results reproducible and not influenced by hidden oracles?

### Mode 7.1: Non-Determinism / Seeded RNG

**Violates**: "Random number generators must not be seeded for determinism"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Hardcoded seed in RNG | Code: `SmallRng::from_seed(SEED)` | CHEAT-001 | Yes |
| Seed derived from time | Code: `seed_from_u64(now().as_secs())` | CHEAT-001 | Yes |
| Seed derived from fixed hash | Code: `seed = hash("constant")` | CHEAT-001 | Yes |
| Seed hardcoded in test | Test: `rng = seeded_rng(42)` | CHEAT-001 | Yes |
| Random state not reset | Test: RNG state persists across tests | CHEAT-001 | Yes |

**Severity**: BLOCKING (breaks reproducibility)

---

### Mode 7.2: Hardcoded Metrics

**Violates**: "Metrics must be computed, not hardcoded"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Constant returned as measurement | Code: `fn conformance() { 100 }` | CHEAT-002 | Yes |
| Metric hardcoded to "lucky" number | Code: `return 42` or `return 99` | CHEAT-002 | Yes |
| Synthetic metric computed from environment | Code: `env::var("CONFORMANCE")` | CHEAT-002 | Yes |
| Metric computed once, cached forever | Code: `lazy_static! { METRIC = compute() }` | CHEAT-002 | Yes |
| Metric conditioned on CLI argument | Code: `if args.get("--pass") { 100 }` | CHEAT-002 | Yes |
| Metric missing when run in test vs CI | Behavior: passes locally, fails in CI | CHEAT-002 | Yes |

**Severity**: BLOCKING (breaks integrity)

---

### Mode 7.3: Oracle / Transmute

**Violates**: "No unsafe type coercion or memory manipulation"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| `mem::transmute` used | AST: `transmute` function call | ORACLE-001 | Yes |
| Pointer casting | AST: `as *const T` or `as *mut T` | ORACLE-001 | Yes |
| Unsafe block with pointer arithmetic | AST: `unsafe { }` with `*p` | ORACLE-001 | Yes |
| Type erasing via `Any` trait | Code: `as_any().downcast()` | ORACLE-001 | Yes |
| Undefined behavior (UB) via unchecked ops | Code: `unchecked_mul`, `assume` | ORACLE-001 | Yes |

**Severity**: BLOCKING (breaks type safety)

---

### Mode 7.4: Environment-Dependent Behavior

**Violates**: "Behavior must not depend on environment variables (except explicit inputs)"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Behavior depends on `$HOME` | Code: `env::var("HOME")` used in logic | ORACLE-002 | Yes |
| Behavior depends on `$PATH` | Code: `env::var("PATH")` used to find binary | ORACLE-002 | Yes |
| Behavior depends on `$DEBUG` | Code: `env::var("DEBUG")` gates feature | ORACLE-002 | Yes |
| Behavior depends on hostname | Code: `hostname::get()` used in logic | ORACLE-002 | Yes |
| Behavior depends on system load | Code: `load_average()` affects branching | ORACLE-002 | Yes |
| Behavior depends on process memory | Code: `memory_usage()` affects logic | ORACLE-002 | Yes |
| Behavior depends on external service | Network call to external API | ORACLE-002 | Yes |

**Severity**: BLOCKING (breaks reproducibility)

---

### Mode 7.5: Memoization / Lookup Tables

**Violates**: "Lookup table values must be computed, not hardcoded"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Hardcoded lookup table | Code: `const LOOKUP: &[u32] = &[1, 2, 3, ...]` | ORACLE-003 | Yes |
| Lazy-static memo table | Code: `lazy_static! { static MEMO = HashMap::from(...) }` | ORACLE-003 | Yes |
| Precomputed hash values | Code: `const HASH_TABLE = [0x123, 0x456, ...]` | ORACLE-003 | Yes |
| Cached result with no invalidation | Code: `once_cell::sync::Lazy::new(compute)` | ORACLE-003 | Yes |

**Severity**: BLOCKING (breaks recomputation principle)

---

## Dimension 8: CLAIMS & LANGUAGE (Victory, Overclaiming)

**Axis**: Does language reflect actual proof, or make unsupported claims?

### Mode 8.1: Victory Language

**Violates**: "No victory language without evidence"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| "fully solved" | Text: `contains("fully solved")` | CLAIM-004 | Yes |
| "all clean" | Text: `contains("all clean")` | CLAIM-004 | Yes |
| "done" (in claims context) | Text: `contains("is done")` in doc/comment | CLAIM-004 | Yes |
| "guaranteed" | Text: `contains("guaranteed")` | CLAIM-004 | Yes |
| "solved" | Text: `contains("solved")` | CLAIM-004 | Yes |
| "fixed" (absolute) | Text: `contains("is fixed")` without qualifier | CLAIM-004 | Yes |
| "working" (absolute) | Text: `contains("is working")` without caveats | CLAIM-004 | Yes |
| "impossible to fake" | Text: `contains("impossible")` | CLAIM-004 | Yes |
| "fully admitted" (if not domain term) | Text: `contains("fully admitted")` | CLAIM-004 | Yes |

**Severity**: BLOCKING (without domain exemption)

---

### Mode 8.2: Overclaiming

**Violates**: "Claims must match proof level"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| "Supports LSP 3.18 feature X" with BLOCKED receipt | Claim: supported, receipt: BLOCKED | CLAIM-004 | Yes |
| "Zero violations" when CANDIDATE violations exist | Claim: zero, status: CANDIDATE | CLAIM-004 | Yes |
| "Fully compliant" when missing receipts | Claim: fully, receipts: missing | CLAIM-004 | Yes |
| "100% coverage" when untested code exists | Claim: 100%, coverage: 99% | CLAIM-004 | Yes |
| "All rules passed" with warnings present | Claim: all, diagnostics: warnings exist | CLAIM-004 | Yes |

**Severity**: BLOCKING

---

## Dimension 9: COMPLEXITY & CODE SMELLS

**Axis**: Does code complexity hide cheating?

### Mode 9.1: Function Size

**Violates**: "Functions > 500 LOC likely hide multiple concerns"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Function > 500 lines | AST: function line count | METRIC-001 | No (warning) |
| Function > 1000 lines | AST: function line count | METRIC-001 | Yes |
| Nested function > 200 lines | AST: inner function size | METRIC-001 | No (warning) |
| Function with >30 parameters | AST: parameter count | METRIC-001 | No (warning) |

**Severity**: WARNING → BLOCKING (depending on size)

---

### Mode 9.2: Cyclomatic Complexity

**Violates**: "High complexity hides control flow"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Cyclomatic complexity > 15 | Control flow analysis | METRIC-002 | No (warning) |
| Cyclomatic complexity > 30 | Control flow analysis | METRIC-002 | Yes |
| Deeply nested conditions (> 5 levels) | AST: nesting depth | METRIC-002 | No (warning) |
| Too many branches (> 20 `if`s) | AST: branch count | METRIC-002 | No (warning) |

**Severity**: WARNING → BLOCKING

---

### Mode 9.3: Literal Table Proliferation

**Violates**: "Large literal tables are suspicious (may hide computations)"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Literal array with > 100 elements | AST: array size | METRIC-003 | No (warning) |
| Literal map with > 50 entries | AST: map size | METRIC-003 | No (warning) |
| Hardcoded matrix (2D array) | AST: nested arrays | METRIC-003 | No (warning) |
| Hardcoded lookup table for computed value | AST + logic: detects suspicion | METRIC-003 | Maybe (context-dependent) |

**Severity**: WARNING

---

## Dimension 10: LSP 3.18 Feature Claims

**Axis**: Are LSP 3.18 features actually implemented and admitted?

### Mode 10.1: Feature Matrix Violations

**Violates**: "Features must have transcript, receipt, and test"

**Patterns**:

| Pattern | Detection | Code | Blocking |
|---------|-----------|------|----------|
| Feature claimed SUPPORTED with no transcript | LSP318 matrix: transcript column empty | LSP318-NNN | Yes |
| Feature claimed SUPPORTED with no receipt | LSP318 matrix: receipt column empty | LSP318-NNN | Yes |
| Feature claimed SUPPORTED with CANDIDATE receipt | LSP318 matrix: CANDIDATE status | LSP318-NNN | Yes |
| Feature claimed SUPPORTED with negative control missing | LSP318 matrix: negative control column empty | LSP318-NNN | Yes |
| Feature status = REFUSED but feature in code | Code/matrix mismatch | LSP318-NNN | Yes |
| Feature marked BLOCKED without explanation | LSP318 matrix: reason column empty | LSP318-NNN | Yes |

**Severity**: BLOCKING

---

## Summary Table: All Dimensions

| Dimension | Modes | Example Violations | Blocking? |
|-----------|-------|-------------------|----------|
| **SURFACE** | 3 | tower-lsp refs, capabilities, mutations | Yes |
| **AUTHORITY** | 3 | CLAP fakes, string authority, routing | Yes |
| **RECEIPT** | 3 | Fake receipts, missing evidence, ledger breaks | Yes |
| **ROUTE** | 3 | Log-as-proof, static analysis, invisible I/O | Yes |
| **MUTATION** | 3 | Direct fs, WorkspaceEdit misuse, side effects | Yes |
| **VERSION** | 2 | CalVer, temporal anomalies | Yes |
| **DETERMINISM** | 5 | Seeded RNG, hardcoded metrics, oracles, env vars, memos | Yes |
| **CLAIMS** | 2 | Victory language, overclaiming | Yes |
| **COMPLEXITY** | 3 | Function size, cyclomatic, literals | Sometimes |
| **LSP 3.18** | 1 | Feature matrix gaps | Yes |

**Total violation patterns**: ~500+ distinct

---

## Detection Strategy

### Orthogonal Scanning

Each violation dimension is scanned independently:

```
Rule modules (lsp-max-anti-cheat):
├── rules/surface.rs ────────────→ SURFACE-*
├── rules/authority.rs ──────────→ AUTH-*
├── rules/receipts.rs ───────────→ RECEIPT-*
├── rules/routes.rs ─────────────→ ROUTE-*
├── rules/mutation.rs ───────────→ MUT-*
├── rules/version.rs ────────────→ VERSION-*
├── rules/determinism.rs ────────→ CHEAT-*, ORACLE-*
├── rules/claims.rs ─────────────→ CLAIM-*
├── rules/complexity.rs ─────────→ METRIC-*
└── rules/lsp318.rs ─────────────→ LSP318-*
```

Each rule module:
1. Scans input (observations from parsers)
2. Checks for violation patterns
3. Emits diagnostics (code, category, blocking)
4. Never leaks state between runs

### Parsers (Input Layer)

Observations are produced by specialized parsers:

```
Parsers (lsp-max-anti-cheat/src/parsers):
├── rust_tree_sitter.rs ────→ AST nodes, symbols, unsafe blocks
├── cargo_toml.rs ──────────→ Manifest, versions, dependencies
├── cargo_lock.rs ──────────→ Locked versions, transitive deps
├── markdown_claims.rs ─────→ Text patterns, language
├── json_rpc.rs ────────────→ LSP transcripts, capabilities
├── receipt_json.rs ────────→ Receipt structure, BLAKE3 hashes
├── ggen_toml.rs ───────────→ Code generation config
├── typescript.rs ──────────→ JS/TS code patterns
└── contract.rs ────────────→ Vocabulary consistency
```

Each parser produces `Observation` structs with:
- `file_path`, `line`, `column`
- `kind` (raw_text, ast_node, manifest_dep, etc.)
- `construct` (what was found: "tower-lsp", "unwrap()", etc.)
- `context` (surrounding code/text)

---

## Achieving Combinatorial Completeness

**Principle**: Never discover a new cheat pattern in the field.

**Method**: 
1. Enumerate all violation dimensions (10 above)
2. Enumerate all modes within each dimension
3. Enumerate all patterns within each mode
4. Implement detection for each pattern
5. Test with negative controls (36+ fixtures)

**Result**: ~500+ distinct violations covered; agent cannot invent new cheats faster than rules detect them.

---

## Implications for Agents

1. **No hiding in complexity** — Code size, nesting, literals all trigger warnings
2. **No silent side effects** — All mutations and I/O must be traced
3. **No synthetic proof** — All claims must have receipts
4. **No determinism tricks** — Seeding, memos, hardcoded values all detected
5. **No time tricks** — CalVer law and monotonic timeline enforced

**Conclusion**: **Honest work is easier than cheating.**

---

## References

- CLAUDE.md: Law-state semantics and violation categories
- ANTI-LLM-CHEAT-LSP: Detection implementation (80+ diagnostic codes)
- LSP 3.18 Specification: Capability semantics
- OCEL Standard: Event log format for audit trails

---

**Status**: CANDIDATE (enumeration complete; implementation ongoing with anti-cheat rules)
