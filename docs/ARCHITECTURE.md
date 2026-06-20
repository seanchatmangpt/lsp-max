# lsp-max Architecture

## Overview

**lsp-max** is a "law-state runtime projected through LSP" — a five-layer stack that enforces protocol-backed invariants at every boundary. Unlike plain tower-lsp (which is LSP *servers* for humans), lsp-max is LSP *for agents*, with the repository itself speaking back to agents through bounded receipts, conformance vectors, repair intents, and gate enforcement.

The five-layer model is **law-state as the load-bearing abstraction**:

```
Layer 5: Autonomic LSP Mesh (conformance, admission gates, repair)
         ↓
Layer 4: Knowledge Hooks (composition, routing, capability tracking)
         ↓
Layer 3: Law-State Runtime (typestate, phases, receipts, SHA256 chains)
         ↓
Layer 2: Local LSP State Surface (LanguageServer trait, poll_ready, stdio/TCP)
         ↓
Layer 1: Actuation Grammar (clap-noun-verb CLI: noun=filename, verb=action)
```

Each layer enforces invariants that make the next layer's claims trustworthy. No layer is passive; all are law-bearing.

---

## Layer 1: Actuation Grammar — clap-noun-verb CLI

**Crate**: `crates/lsp-max-cli`

**Entry point**: `main.rs` → `clap_noun_verb::run()`

### Purpose

The actuation grammar is the interface between agents and the law-state runtime. It is a **noun/verb command grammar** where:

- **Nouns** are filenames in `crates/lsp-max-cli/src/nouns/` (e.g., `server.rs`, `conformance.rs`, `state.rs`, `diagnostics.rs`)
- **Verbs** are `#[verb]` attributes on public methods within those nouns
- **Actions** are bounded and routed through the runtime's hook and receipt system

### Key Design

```rust
// Example: crates/lsp-max-cli/src/nouns/server.rs
#[verb]
pub async fn start(&self, host: String, port: u16) -> Result<ServerDetails> {
    // Fetch current mesh state, check gates, route through hooks, emit receipt
}
```

Each command:
1. Reads the current server state from the persistent mesh (JSON or embedded database)
2. Evaluates law gates (`gate.rs`) to determine if the action is lawful
3. Dispatches the action through the autonomic mesh (Layer 3)
4. Emits a receipt binding the action, its outcome, and the next state
5. Returns a result or error to the agent

### Invariant: Deterministic Actuation

**No state mutations occur without a receipt.**

- Mutations are staged in `Staging` before the `MutationGate` admits them
- The gate checks `Λ_CD(action)` — whether the action satisfies all law axes
- Only after gate admission is the mutation written to the persistent mesh
- The receipt binds: `action_id → outcome → next_state → SHA256(state_transition)`

### Nouns (Actuation Surface)

- **`server.rs`** — Lifecycle: `start`, `stop`, `reload`, `query`
- **`state.rs`** — Inspection: `state` (instance phase, conformance, diagnostics count)
- **`diagnostics.rs`** — Diagnostic manipulation: `diagnose`, `clear`, `run`, `list`
- **`conformance.rs`** — Conformance queries: `score`, `vector`, `delta`
- **`snapshot.rs`** — Snapshotting: `snapshot`, `restore`
- **`agent.rs`** — Agent integration: route analysis bundles, bound actions
- **`workspace.rs`** — Workspace inspection: list instances, query composition
- **`metamodel.rs`** — LSP 3.18 spec inspection: capabilities, methods, transcripts

### File Structure Invariant

Files stay ≤500 LOC; split into submodules matching the noun name:

```
crates/lsp-max-cli/src/nouns/
├── server.rs         (domain structs + service tier + verb handlers)
├── server/
│   ├── mod.rs        (impl details, private types)
│   └── ...
└── ...
```

---

## Layer 2: Local LSP State Surface

**Crates**: `src/` (root), `crates/lsp-max-client`

**Key types**: `LanguageServer` trait, `LspService`, `Server`, `Client`

### Purpose

Layer 2 is the **LSP protocol transport and method routing layer**. It handles:

1. **Transport**: stdio, TCP, loopback (in-process testing)
2. **Method registration**: via `LanguageServer` trait and `#[async_trait]`
3. **State machine**: `State` enum (Uninitialized → Initializing → Initialized → ShutDown → Exited)
4. **Lifecycle coordination**: `poll_ready`, request/response correlation, cancellation
5. **Message framing**: JSON-RPC 2.0 codec (Content-Length headers, async frames)

### Key Structures

#### LanguageServer Trait

```rust
#[lsp_max::async_trait]
pub trait LanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult>;
    async fn initialized(&self, params: InitializedParams);
    async fn shutdown(&self) -> Result<()>;
    // ... 100+ standard LSP methods
    // ... custom max/* methods via router
}
```

The trait is object-safe; servers are boxed as `Box<dyn LanguageServer>`.

#### LspService

```rust
pub struct LspService<S: LanguageServer> {
    inner: layers::CatchUnwindService<Router<S, ExitedError>>,
    state: Arc<ServerState>,
}

impl<S: LanguageServer> Service<Request> for LspService<S> {
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Returns Pending until initialization completes.
        // After Exited, returns Err(ExitedError).
    }

    fn call(&mut self, req: Request) -> Self::Future {
        // Routes req through method dispatch, returns future of Option<Response>
    }
}
```

**Key invariant**: `poll_ready` returns `Poll::Pending` while `State == Initializing`; the service cannot serve requests until the server finishes initialization.

#### Server (Transport Multiplexer)

```rust
pub struct Server<I, O, L = ClientSocket> {
    stdin: I,
    stdout: O,
    loopback: L,
    max_concurrency: usize,
}

impl<I, O, L> Server<I, O, L> {
    pub async fn serve<T: Service<Request>>(self, mut service: T) -> Result<(), T::Error> {
        // 1. Frame stdin via LanguageServerCodec (Content-Length headers)
        // 2. Multiplex N concurrent requests via buffer_unordered
        // 3. Route responses back through stdout frames
        // 4. Detect service exit via probe_exited and exit state
    }
}
```

The server is **transport-agnostic** — it works with any `AsyncRead`/`AsyncWrite`, including loopback channels for tests.

### Data Flow (Receive Path)

```
[Client] 
  ↓ (JSON-RPC 2.0 frame)
stdin → FramedRead::decode(Content-Length header + body)
  ↓
Server::serve() → LanguageServerCodec::decode()
  ↓
Request (method, params, id)
  ↓
Service::call(req)
  ↓
LanguageServer method dispatch (via Router)
  ↓
Handler method (async, on the backend struct)
```

### Data Flow (Send Path)

```
Handler returns Result<T>
  ↓
to_value(T) → serde_json::Value
  ↓
Response { id, result/error }
  ↓
Service returns Option<Response>
  ↓
Server::serve() queues Response
  ↓
FramedWrite::encode(Content-Length header + body)
  ↓
stdout → [Client]
```

### Invariants

1. **poll_ready is gating**: A service cannot receive requests until initialization succeeds.
2. **State is monotonic**: Uninitialized → Initializing → Initialized → ShutDown → Exited (no back-edges).
3. **stdout is for framing only**: No debug prints, logs, or diagnostic output on stdout — use `eprintln!` or `tracing` (stderr) instead.
4. **Method not found is silent**: `$/unknownMethod` errors are filtered and converted to `None` before returning.

---

## Layer 3: Law-State Runtime

**Crates**: `lsp-max-runtime`, `lsp-max-protocol`

**Key types**: `AutonomicMesh`, `MaxDiagnostic`, `ConformanceVector`, `Receipt`, `TypestateKernel`

### Purpose

Layer 3 is the **proof system and state machine**. It:

1. **Enforces law invariants** through gate evaluation (`Λ_CD`)
2. **Tracks phases** via a typestate machine (Uninitialized, Initializing, Initialized, ShutDown, Exited)
3. **Records transitions** with SHA256-signed receipts binding (from_state → action → to_state)
4. **Maintains conformance deltas** as a ring-buffer log for `max/conformanceDelta` polling
5. **Routes diagnostics and repairs** through a composable hook system

### The Autonomic Mesh

```rust
pub struct AutonomicMesh {
    instances: HashMap<InstanceId, LspInstance>,
    hooks: Vec<Box<dyn Hook>>,
    events: Vec<HookEvent>,  // Ring buffer, capped at 1000 entries
    conformance_delta_log: VecDeque<ConformanceDeltaEntry>,
}

impl AutonomicMesh {
    pub fn dispatch_rpc(
        &mut self,
        instance_id: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        // 1. Create HookEvent from (method, params)
        // 2. Dispatch through hook chain
        // 3. Apply MeshActions returned by hooks
        // 4. Update instance state (diagnostics, receipts, phase)
        // 5. Return result or error
    }

    pub fn register_hook(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
    }
}
```

### Hook System (Composition Filter Pipeline)

Hooks intercept method calls and emit actions:

```rust
pub trait Hook: Send + Sync {
    fn name(&self) -> &'static str;
    fn descriptor(&self) -> HookDescriptor;
    
    fn on_event(&self, event: &HookEvent) -> Vec<MeshAction>;
}

pub enum MeshAction {
    RegisterDiagnostic(MaxDiagnostic),
    ClearDiagnostic(String),           // diagnostic_id
    TransitionPhase(LspPhase),
    EmitReceipt(Receipt),
    UpdateConformanceScore(f64),
}
```

### Standard Hooks (Built During Mesh Construction)

1. **`IntakeDiagnosticHook`** — Routes `max/diagnostic` calls into the registry
2. **`IntakeClearHook`** — Routes `max/clear` calls to remove diagnostics
3. **`CustomerRequestClassifierHook`** — Categorizes method calls (request, notification, custom)
4. **`PolicyEvaluationHook`** — Evaluates gate conditions; emits phase transitions
5. **`ReceiptRoutingHook`** — Binds transitions to SHA256-signed receipts
6. **`OcelProcessHook`** — (Optional) Event-sourced process mining via wasm4pm

### Receipt Structure

```rust
pub struct Receipt {
    pub receipt_id: String,               // "rcpt-sha256-digest"
    pub timestamp: String,                // RFC-3339
    pub from_phase: LspPhase,
    pub to_phase: LspPhase,
    pub action_id: String,                // The admitted action
    pub law_axes_consulted: Vec<LawAxis>, // law_intake_validation, law_admissibility, ...
    pub digest: String,                   // SHA256(JSON canonical form)
    pub chain_link: Option<String>,       // Previous receipt digest (forms a chain)
}
```

Receipts are **immutable and append-only**. They form an ordered chain: each receipt's `chain_link` points to the previous receipt's digest, creating a tamper-evident log.

### Typestate Machine

```rust
pub struct TypestateKernel<Phase: crate::Law> {
    data: Phase::Data,
}

// Witness types for each phase
pub struct Uninitialized;
pub struct Initializing;
pub struct Initialized;
pub struct ShutDown;
pub struct Exited;

// Law trait encoding transitions
pub trait Law: Sized {
    type Data: Send + Sync;
    // Allowed transitions: Initializing, Initialized, ShutDown, Exited
    fn transition(self, action: Action) -> Result<NextPhase, Error>;
}
```

The typestate machine is **compile-time proof that only valid phase transitions are possible**. Invalid transitions (e.g., Exited → Initialized) cannot be expressed.

### ConformanceVector

```rust
pub struct ConformanceVector {
    pub admitted: Vec<LawAxis>,           // Law axes that are satisfied
    pub refused: Vec<LawAxis>,            // Law axes that are violated (Error severity)
    pub unknown: Vec<LawAxis>,            // Law axes that are incomplete
    pub score: Option<f64>,               // 100.0 * |admitted| / (|admitted| + |refused|)
    pub strict_mode: bool,                // Fail on Unknown
}
```

**Invariant**: `Unknown` never collapses into `Admitted` or `Refused`. When a law axis cannot be evaluated, it remains `Unknown` — this signals a gap, not success.

### Global Registry

```rust
pub struct ServerRegistry {
    pub client_capabilities: Option<ClientCapabilities>,
    pub server_capabilities: Option<ServerCapabilities>,
    pub diagnostics: HashMap<String, MaxDiagnostic>,      // diagnostic_id → diagnostic
    pub repair_plans: HashMap<String, Vec<MaxCodeAction>>, // uri → repair actions
    pub gates: HashMap<String, bool>,
    pub receipts: HashMap<String, Receipt>,
    pub snapshots: HashMap<String, SnapshotRecord>,
    pub current_state: State,
    pub action_seq: u64,                  // Monotonic, increments on every release
    pub conformance_delta_log: VecDeque<ConformanceDeltaEntry>,
}

pub static REGISTRY: OnceLock<Mutex<ServerRegistry>> = OnceLock::new();
pub static MESH: OnceLock<Mutex<AutonomicMesh>> = OnceLock::new();
```

Both are **global singletons** initialized on first access. A future RFC proposes replacing these with per-connection `ServerSession<S>` to enable true multi-tenancy.

### Invariants

1. **Receipts are immutable**: Once emitted, a receipt cannot be modified or deleted.
2. **Chain is tamper-evident**: Each receipt links to its predecessor's SHA256 digest. A missing or invalid link signals tampering.
3. **Diagnostics are transient**: They live in `registry.diagnostics` and can be cleared, but cleared IDs are tracked in `cleared_diagnostics` to prevent re-admission of the same issue.
4. **State transitions require gates**: Before transitioning phases, `accept_gates(registry, gates)` must return true.
5. **Conformance deltas are ordered**: The `conformance_delta_log` is ordered by `seq` (action sequence number). Polling clients provide a `since_seq` cursor; results are all entries `> since_seq`.

---

## Layer 4: Knowledge Hooks — Composition and Routing

**Crate**: `src/composition/`

**Key modules**: `routing.rs`, `capability_tracker.rs`, `merge.rs`, `upstream.rs`, `edit_gate.rs`, `server.rs`

### Purpose

Layer 4 is the **multi-server composition and knowledge inference layer**. It:

1. **Discovers and merges capabilities** from N child LSP servers
2. **Routes methods** to the appropriate tier (Primary, Secondary, DiagnosticsOnly)
3. **Merges responses** (e.g., hovers, completions, diagnostics) with deduplication
4. **Tracks document versions** to enforce causality (a document edit must reference a known version)
5. **Guards mutations** via `TransactionEditGate` — a WorkspaceEdit is admitted only if all text positions can be validated against known document versions

### Composition Tier System

```rust
pub enum ChildTier {
    Primary,           // Hover, completion, definition: use only primary
    Secondary,         // Fallback if primary fails
    DiagnosticsOnly,   // Emit diagnostics but not code actions or hovers
}

pub struct ChildServer {
    id: String,
    tier: ChildTier,
    process_handle: Child,
    connected: bool,
    capabilities: ServerCapabilities,
}
```

**Routing invariant**:

```
textDocument/hover, definition, completion → Primary only (FirstSuccess)
textDocument/publishDiagnostics              → All tiers (FanAll, REFUSED_BY_LAW survives merge)
unknown methods                              → Primary only (conservative)
```

### Capability Merging

```rust
pub struct CapabilityTracker {
    primary_caps: ServerCapabilities,
    secondary_caps: Vec<ServerCapabilities>,
    diagnostics_only_caps: Vec<ServerCapabilities>,
}

impl CapabilityTracker {
    pub fn merged_capabilities(&self) -> ServerCapabilities {
        // Intersection for hover_provider, completion_provider (primary wins)
        // Union for diagnostics, formatting
        // ExcludedMethods for DiagnosticsOnly servers
    }
    
    pub fn supports(&self, method: &str) -> bool {
        // Does merged capability set include this method?
    }
}
```

### Document Version Tracking

```rust
pub struct DocumentVersionTracker {
    versions: HashMap<Url, i32>,  // Document URI → current version
}

impl DocumentVersionTracker {
    pub fn on_did_open(&mut self, uri: Url, version: i32) {
        self.versions.insert(uri, version);
    }
    
    pub fn on_did_change(&mut self, uri: Url, version: i32) {
        // Only update if version > current version (monotonic)
    }
    
    pub fn check_position(&self, uri: &Url, version: i32) -> VersionCheckResult {
        // Known, Ahead, or Behind (for WorkspaceEdit validation)
    }
}
```

### Transaction Edit Gate

```rust
pub struct TransactionEditGate {
    doc_versions: DocumentVersionTracker,
}

pub enum EditGateOutcome {
    Admitted,
    Refused(String),  // "position (5, 10) is ahead of known version 2"
}

impl TransactionEditGate {
    pub fn validate_workspace_edit(
        &self,
        edit: &WorkspaceEdit,
    ) -> EditGateOutcome {
        // For each TextDocumentEdit:
        // - Extract URI and all (line, character) tuples
        // - Check against doc_versions
        // - Reject if any position is ahead of known version
        // - Reject if any range spans multiple unknown documents
    }
}
```

### Response Merging

```rust
pub fn merge_hovers_with_attribution(
    hovers: Vec<(ServerId, Option<Hover>)>,
) -> Option<Hover> {
    // Primary hover wins; if absent, take secondary; DiagnosticsOnly excluded
    // Source is attributed: response includes [comment: "from server_id"]
}

pub fn merge_workspace_edits(
    edits: Vec<(ServerId, WorkspaceEdit)>,
) -> WorkspaceEdit {
    // Dedup by URI; coalesce edits to the same range
    // Reject if version markers conflict
}
```

### Invariants

1. **Primary is authority**: For hovers/completions, primary server's response is used; secondary is fallback.
2. **Version causality**: A document cannot be edited at a position newer than the known version.
3. **Merges are conservative**: If a merge would require conflicting information (e.g., two servers disagree on range boundaries), the merge is refused.
4. **DiagnosticsOnly is read-only**: These servers can emit diagnostics but cannot influence code actions or hovers.

---

## Layer 5: Autonomic LSP Mesh — Conformance and Gates

**Crates**: `crates/lsp-max-compositor`, `examples/anti-llm-cheat-lsp`

**Key types**: `CompositorClient`, `MergeContext`, `GateFile`, `CompositorReceipt`, `Λ_CD` predicate

### Purpose

Layer 5 is the **autonomic mesh layer** — the "law-state runtime projected through LSP." It:

1. **Spawns and reaps child LSP processes** with process monitoring and exit watchers
2. **Fans out client requests** to all child servers in parallel
3. **Merges diagnostics** with ANDON signal handling (Red/Yellow/Green based on conformance)
4. **Emits conformance vectors** tracking admitted vs. refused law axes
5. **Enforces admission gates** via the `Λ_CD` predicate (no shell/file action while ANDON is set)
6. **Detects violations** (plain tower-lsp, victory language, fake receipts) via anti-llm-cheat-lsp canary

### Compositor Architecture

```rust
pub struct CompositorClient {
    config: CompositorConfig,
    children: HashMap<ServerId, PersistentUpstream>,
    merge_context: MergeContext,
    gate_file: GateFile,
}

impl CompositorClient {
    pub async fn initialize(&mut self) -> Result<()> {
        // 1. Load lsp-max.toml (or default config)
        // 2. Spawn child processes for each server entry
        // 3. Connect to each child via stdio LSP
        // 4. Exchange initialize/initialized
        // 5. Merge capabilities
        // 6. Return merged capabilities to client
    }

    pub async fn on_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        // 1. Route method based on tier and capability
        // 2. Send to child(ren) in parallel
        // 3. Merge responses
        // 4. Check ANDON gate
        // 5. Emit receipt
        // 6. Return result
    }

    pub async fn on_notification(&mut self, method: &str, params: Value) -> Result<()> {
        // Fan-out to all children; collect, check ANDON, emit receipt
    }
}
```

### Diagnostic Buffer and Flush Coordinator

```rust
pub struct DiagnosticBuffer {
    // DashMap<Uri, Vec<(ServerId, Diagnostic)>>
    // Non-destructive deposits: deposit(uri, server_id, diag) appends
    // flush() atomically returns and clears all entries
}

pub struct FlushCoordinator {
    // Debounce buffer: 100ms (or quorum-triggered)
    // Emits CompositorReceipt after each flush
}

pub struct CompositorReceipt {
    pub timestamp: String,
    pub prefixes_fingerprint: String,  // FNV-1a hash of ANDON prefix set
    pub has_andon_block: bool,         // Any REFUSED_BY_LAW present?
    pub child_evidence: Vec<ChildEvidence>, // Links to per-child receipts
}
```

### ANDON Gate (Λ_CD^runtime)

```rust
pub struct GateFile {
    // Path: $XDG_RUNTIME_DIR/lsp-max-gate-{fnv1a(cwd):016x}
    // Content: b"0" (clear) or b"1" (BLOCKED)
}

impl GateFile {
    pub fn check(&self) -> Result<()> {
        // Read one byte; return error if b"1"
        // Used by PreToolUse hook: lsp-max-cli gate check before every Bash/Edit/Write
    }

    pub fn set_andon(&self) -> Result<()> {
        // Write b"1" when first ANDON-class diagnostic appears
        // Used by compositor after diagnostic merge
    }

    pub fn clear_andon(&self) -> Result<()> {
        // Write b"0" when no ANDON-class diagnostics remain
    }
}
```

**Λ_CD predicate**:

```
Λ_CD(action) = Λ(action) ∧ ¬∃ d ∈ D_t : d.law_id ∈ A ∧ d.severity = Error

Where:
- Λ(action): agent's base admissibility (receipts, no victory language, no forbidden implications)
- D_t: active diagnostics at time t
- A: constrained set of law axes (WASM4PM-*, ANTI-LLM-*, GGEN-*)
- d.severity = Error: only Error-level violations block; Warning/Hint do not
```

**Enforcement**:

- **Parent session**: `.claude/settings.json` has a `PreToolUse` hook that runs `lsp-max-cli gate check` before every Bash/Edit/Write tool call
- **Exit code 0**: Gate is clear; tool proceeds
- **Exit code 1**: ANDON is set; tool is blocked until gate clears
- **Subagent sessions**: Structurally isolated from parent hooks (not a configuration error, a structural gap)

### Anti-LLM-Cheat Canary

```rust
// examples/anti-llm-cheat-lsp/src/main.rs
pub struct AntiLlmCheatLsp {
    // Runs on lsp-max (does NOT depend on plain tower-lsp)
    // Detects and emits diagnostics for:
    // - Plain tower-lsp references (ANTI-LLM-SURFACE-001)
    // - Fake authorities (ANTI-LLM-AUTH-002)
    // - Fake receipts (ANTI-LLM-RECEIPT-*) 
    // - Fake routes (ANTI-LLM-ROUTE-001)
    // - Victory language (ANTI-LLM-CLAIM-*)
    // - Version violations (ANTI-LLM-VERSION-*)
}

// Detector stack:
// 1. Raw text scan (grep-based)
// 2. Tree-sitter AST scan (Cargo.toml, Rust code)
// 3. Cargo dependency graph scan
// 4. Markdown/agent report scan
// 5. JSON-RPC transcript scan
// 6. Receipt validator (SHA256, boundary markers, checkpoint closure)
// 7. Route evidence checker (CodeAction → clap-noun-verb → Receipt)
// 8. Claim vs. proof checker
// 9. Emit LSP diagnostics + virtual documents
```

**Self-sealing law**:

```
lsp-max hosts anti-llm-cheat-lsp
anti-llm-cheat-lsp detects tower-lsp
therefore lsp-max cannot silently regress to tower-lsp
```

### Invariants

1. **Diagnostics are ordered and immutable**: Emitted in a single `textDocument/publishDiagnostics` per flush; cannot be reordered after emission.
2. **ANDON blocks shell actions**: While any ANDON-class diagnostic is present, the gate file is b"1" and all Bash/Edit/Write calls are blocked.
3. **Receipts are causal**: `CompositorReceipt` links to child receipts; the merged verdict is traceable to per-child evidence.
4. **Merges are non-destructive**: Diagnostic deduplication never discards REFUSED_BY_LAW codes; they survive the merge and trigger ANDON.

---

## Data Flow: Request → Receipt

An example trace of a single LSP request through all five layers:

### Scenario
Agent calls `lsp-max-cli conformance score --instance-id LSP_1`

### Layer 1: Actuation Grammar
```
[clap-noun-verb parser]
  ↓
"conformance score" → conformance::ScoreParams { instance_id: "LSP_1" }
  ↓
ServerService::score(instance_id)  [in crates/lsp-max-cli/src/nouns/conformance.rs]
  ↓
Read mesh.json from disk
```

### Layer 2: Local LSP State Surface
```
Load ServerRegistry from global REGISTRY singleton
  ↓
Query: registry.current_state, registry.diagnostics, registry.action_seq
  ↓
(No transport here — CLI works directly with in-process registry)
```

### Layer 3: Law-State Runtime
```
Create ConformanceVector from registry.diagnostics
  ↓
admitted = count of non-Error diagnostics
refused = count of Error-severity diagnostics
unknown = 0 (all law axes are known)
  ↓
score = 100.0 * admitted / (admitted + refused)
  ↓
Emit MeshAction::UpdateConformanceScore(score)
  ↓
Hook chain processes action, updates registry.conformance_delta_log
  ↓
Emit Receipt { action_id: "conformance-score-query", from_phase, to_phase, digest }
```

### Layer 4: Composition Knowledge (N/A for this example)
```
(Composition only applies if a CompositorClient is present)
(For direct registry queries, no composition occurs)
```

### Layer 5: Autonomic Mesh (ANDON Check)
```
If ANDON gate is set: refuse the query, emit ANDON error
  ↓
Otherwise: return conformance_vector + receipt
```

### Return to Agent
```
conformance score = 85.5
admitted = 17
refused = 3
unknown = 0
receipt_id = "rcpt-sha256-abc123..."
chain_link = "rcpt-sha256-previous..."
```

---

## Architectural Invariants and Laws

### I1. Layered Isolation

Each layer is isolated from layers > 2 positions away:

```
Layer 5 can call Layer 3 (diagnostics, gates)
Layer 5 cannot directly mutate Layer 2 (that goes through Layer 3)
Layer 3 can call Layer 2 (state machine, service lifecycle)
Layer 3 cannot directly call Layer 1 (actions route through Layer 2)
```

### I2. Receipt Binding

**No state mutation without a receipt.**

Every action that changes server state must:
1. Originate from Layer 1 (actuation grammar)
2. Pass through Layer 3 (law gates)
3. Be recorded in a Receipt (Layer 3)
4. Be linked in a SHA256 chain (Layer 3)
5. Be reflected in conformance deltas (Layer 3)

### I3. Monotonic State and Phase

The LSP state machine is **monotonic** — only forward transitions are allowed:

```
Uninitialized → Initializing → Initialized → ShutDown → Exited
                    ↓              ↓
                (can block here)  (can block here)
```

No state is ever re-entered. This is **enforced at compile time** via the typestate machine.

### I4. Diagnostics vs. Actions

- **Diagnostics** are **observations** emitted by Layer 3 and collected by Layer 5 (read-only)
- **Actions** are **mutations** initiated by Layer 1 and gated by Layer 3 (write-only, receipt-binding required)

Forbidden implication: `LSP observation ⇒ mutation authority`

### I5. Law Axes Are Not Collapsed

A `ConformanceVector` carries three disjoint sets: `admitted`, `refused`, `unknown`.

- `admitted` means the axis is satisfied with a receipt
- `refused` means the axis is violated (Error-severity diagnostic)
- `unknown` means the axis cannot be evaluated (incomplete data, missing receipt, or precondition not met)

**Invariant**: `unknown` is never coerced into `admitted` or `refused`. To "clear" an `unknown` axis, a receipt must be obtained; otherwise it remains `unknown` indefinitely.

### I6. LSP is Read-Only by Default

The LSP may **emit**:

```
diagnostics
hovers
code action intents
inline completions
virtual documents
command tooltips
failset summaries
protocol traces
```

It must **not directly mutate files**. Future mutations route only through:

```
CodeAction
  ↓
clap-noun-verb admission
  ↓
PackActionIntent
  ↓
PackPlan
  ↓
Staging
  ↓
MutationGate  [checks Λ_CD(action)]
  ↓
Receipt
```

### I7. Composition is Non-Destructive

When merging responses from N child servers, the merge is **non-destructive**:

- Diagnostic merge preserves all REFUSED_BY_LAW codes
- Response merge dedupes by value, not by source
- Capability merge takes intersection (hover, completion) or union (diagnostics)

---

## Testing and Verification

### Integration Tests

Tests live in `tests/` at the workspace root and in per-crate `tests/` directories.

```bash
cargo test --test test_lsp318_capabilities    # Root integration test
cargo test -p anti-llm-cheat-lsp --test dogfood  # Example crate dogfood suite
cargo test -p lsp-max-compositor               # Compositor tests
```

### Dogfood Tests

Dogfood tests are **negative-control tests** that verify the framework detects its own violations:

```rust
#[test]
fn test_gc004b_no_tower_lsp_lock() {
    // Verify that if plain tower-lsp were re-introduced, the canary detects it
    // This test fails if the detection logic is broken
}

#[test]
fn test_receipt_chain_integrity() {
    // Emit actions, verify chain links are correct, SHA256 digests match
}
```

### Conformance Scoring

Every LSP 3.18 feature row requires:

- **Positive transcript**: Raw JSON-RPC messages showing the feature working
- **Negative control**: A case where the feature is refused, with receipt proof
- **Receipt**: SHA256-signed proof that the feature was tested and passed/refused

Status values:

```
SUPPORTED_WITH_TRANSCRIPT
REFUSED_BY_LAW_WITH_RECEIPT
BLOCKED
```

Never: "probably supported", "implied", "covered by normal LSP", "not relevant", "not tested".

---

## Debugging and Troubleshooting

### WASM4PM Build Failures

**Symptom**: `cargo build` fails with "unresolved import `wasm4pm_compat`"

**Check**: Verify sibling checkouts:

```bash
ls ../wasm4pm ../wasm4pm-compat ../lsp-types-max
```

All three must be present in the parent directory (path dependencies).

### Reception Validation Failures

**Symptom**: Diagnostic `ANTI-LLM-CHEAT-LSP-RECEIPT-INVALID` or SHA256 chain mismatch

**Check**: Verify receipt artifact chain:

```bash
scripts/validate-receipt-chain.sh <receipt-path>
```

Look for boundary markers (`-----BEGIN RECEIPT-----`), valid SHA256 digests, and checkpoint closure.

### Conformance Score Anomalies

**Symptom**: A feature row shows low conformance despite code changes

**Check**: Trace admission law axes:

```bash
cargo test test_conformance_vector -- --nocapture
cargo test -p lsp-max-protocol test_law_axis_admission -- --nocapture
```

Identify which law axis is refusing admission (missing transcript, negative control, or unsigned receipt).

### Tower-LSP Reference Detection

**Symptom**: Clippy or `dx-verify` fails with "forbidden: plain tower-lsp reference"

**Check**: Run compliance scanner:

```bash
scripts/check-law-compliance.sh
```

Fix any matches by renaming to `lsp-max` or wrapping in negative-control fixtures.

### Gate State Inspection

**Symptom**: ANDON gate is set, blocking Bash commands

**Check**: Query gate state:

```bash
lsp-max-cli gate check    # Exit 0 = clear; exit 1 = set
lsp-max-cli gate list     # List active WASM4PM-* / GGEN-* diagnostics
```

Resolve all listed diagnostics to clear the ANDON signal.

---

## Crate Dependency Graph

```
crates/lsp-max-cli
  ↓ (depends on)
  lsp-max-protocol
  ↓ (depends on)
  lsp-max-runtime
  ↓ (depends on)
  lsp-max-macros
  ↓ (depends on)
  lsp_types_max  [sibling: ../lsp-types-max]

crates/lsp-max-compositor
  ↓
  lsp-max-protocol, lsp-max-runtime, lsp_types_max

src/ [root]
  ↓
  lsp-max-protocol, lsp-max-runtime, lsp_types_max, lsp-max-macros

examples/anti-llm-cheat-lsp
  ↓
  lsp-max (root crate)
  lsp-max-protocol, lsp-max-runtime

crates/lsp-max-adapters/
  ↓
  lsp-max, lsp-max-protocol
```

---

## Future Directions (RFCs)

### RFC-1: D_t PUSH Injection

**Status**: CANDIDATE

Extend `lsp-max-cli gate check` with `--format=agent-context` flag. When exit 1 (BLOCKED), stdout emits a structured JSON block:

```json
{
  "active_andon_codes": ["WASM4PM-001", "ANTI-LLM-002"],
  "governing_axes": ["wasm4pm", "anti_llm"],
  "available_repairs": [...],
  "since_seq": 42
}
```

The PreToolUse hook injects this as a `<gate-context>` system-reminder block, giving agents full visibility into the governing diagnostic set.

### RFC-2: Per-Connection State

**Status**: OPEN

Replace global `REGISTRY` and `MESH` singletons with a per-connection `ServerSession<S>` struct threaded through the Tower layer as an Extension. Enables true multi-tenancy: N concurrent LSP connections each have isolated D_t, conformance_delta_log, action_seq, and GateFile.

### RFC-3: Event-Sourced D_t Log

**Status**: OPEN

Replace mutable HashMap dispatch with an append-only lock-free ring buffer (65536 entries). Hooks become pure `(LawEvent) -> Vec<MeshAction>` functions. Live D_t is a materialized view maintained by a background tailer. Enables causal replay and D_t addressability.

---

## See Also

- **`AGENTS.md`** — The project constitution; laws are enforced by tooling
- **`CLAUDE.md`** — Development guidance and DX commands
- **`README.md`** — High-level overview
- **`ROADMAP.md`** — Future features and timeline
