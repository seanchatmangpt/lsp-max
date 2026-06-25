# lsp-max protocol reference

Complete reference for the `max/*` protocol, type schemas, gate predicate, LSIF 0.6 extensions,
OCEL event model, Declare constraints, receipt chain, virtual documents, RulePackServer trait,
CLI grammar, and model policies.

---

## max/* methods

All methods are JSON-RPC 2.0. Direction: C→S = client (agent) to server; S→C = server to client
(push notification).

| Method | Dir | Params | Response | When to use |
|--------|-----|--------|----------|-------------|
| `max/snapshot` | C→S | `{timestamp?: string}` | `SnapshotBundle` | Request a point-in-time snapshot of all law-state |
| `max/conformanceVector` | C→S | `{uri?: DocumentUri}` | `ConformanceVector` | Query conformance for one document or workspace |
| `max/workspaceConformance` | C→S | `{strict_mode?: bool}` | `ConformanceVector` | Workspace-wide merged ConformanceVector |
| `max/explainDiagnostic` | C→S | `{uri, code, range}` | `DiagnosticExplanation` | Get prose explanation + law reference for a diagnostic code |
| `max/repairPlan` | C→S | `{uri, diagnosticCode, range}` | `RepairPlan` | Get structured repair transactions for a diagnostic |
| `max/applyRepairTransaction` | C→S | `{plan_id, transaction_index}` | `ApplyResult` | Apply one transaction from a repair plan (witnessed) |
| `max/exportAnalysisBundle` | C→S | `{projectRoot, includeOcel?: bool}` | `AnalysisBundle` | Export full analysis: ConformanceVector + DFG + Declare + receipts |
| `max/runGate` | C→S | `{gate_id?: string}` | `GateResult` | Evaluate gate predicate Λ_CD and return exit code + blocking families |
| `max/clearDiagnostic` | C→S | `{uri, code}` | `ClearResult` | Manually clear a diagnostic (requires receipt proof) |
| `max/receipt` | C→S | `{receipt_id}` | `CryptographicReceipt` | Retrieve a specific receipt by ID |
| `max/hook` | C→S | `{event_type, payload}` | `HookAck` | Fire a hook event into the autonomic loop |
| `max/hookGraph` | C→S | `{}` | `HookGraph` | Return the wired hook graph (event → handler → effect) |
| `max/chain` | C→S | `{discipline_id}` | `ReceiptChain` | Return the full receipt chain for a discipline |
| `max/propagate` | C→S | `{source_uri, target_uri}` | `PropagationResult` | Propagate law-state from one document to another |
| `max/autonomicLoop` | C→S | `{action}` | `AutonomicResult` | Drive the autonomic mesh: `drain_ocel`, `flush`, `checkpoint` |
| `max/manifoldSnapshot` | C→S | `{ts?: string}` | `ManifoldSnapshot` | Snapshot of the full autonomic mesh state |
| `max/lawfulTransition` | C→S | `{from_state, to_state, evidence}` | `TransitionResult` | Assert a typestate transition with evidence |
| `max/admission` | C→S | `{uri?, axes?: LawAxis[]}` | `AdmissionResult` | Query admission status per law axis |
| `max/refusal` | C→S | `{uri, axis, reason}` | `RefusalResult` | Record a refusal with reason (adds to refused set) |
| `max/replay` | C→S | `{ocel_path}` | `ReplayResult` | Replay an OCEL log against the declared process model |
| `max/releaseActuation` | C→S | `{workspace, strict_mode?}` | `ReleaseResult` | Attempt release actuation — blocked if `admits_release()` false |
| `max/rulePacks` | C→S | `{server_id?}` | `RulePackList` | List all loaded rule packs across child servers |
| `max/rulePackStatus` | C→S | `{pack_id}` | `RulePackStatus` | Status of one rule pack: loaded, error, or pending |
| `max/rulePackDiff` | C→S | `{pack_id, version_a, version_b}` | `RulePackDiff` | Diff two versions of a rule pack |
| `max/lsif` | C→S | `{projectRoot, includeConformanceNodes?, outputFormat?}` | NDJSON stream | Export LSIF 0.6 NDJSON; key surface for CI/CD and agent handoffs |

---

## ConformanceVector schema

Defined in `crates/lsp-max-protocol/src/conformance.rs`.

### Fields

| Field | Type | Meaning |
|-------|------|---------|
| `admitted` | `Vec<LawAxis>` | Axes for which law compliance is proven |
| `refused` | `Vec<LawAxis>` | Axes for which law violation is confirmed |
| `unknown` | `Vec<LawAxis>` | Axes not yet traced — neither admitted nor refused |
| `score` | `Option<f64>` | `admitted.len() / (admitted.len() + refused.len() + unknown.len())` when all axes are present |
| `strict_mode` | `bool` | When true, Unknown axes block `admits_release()` |
| `admitted_bits` | `u64` | Bitmask of admitted axes (for fast intersection) |
| `refused_bits` | `u64` | Bitmask of refused axes |
| `unknown_bits` | `u64` | Bitmask of unknown axes |

### LawAxis variants

| Variant | Meaning |
|---------|---------|
| `Protocol` | LSP 3.18 protocol compliance |
| `Type` | Type authority — no forbidden type intermediaries |
| `Fixture` | Test fixtures present and valid |
| `Documentation` | Documentation coverage and accuracy |
| `Release` | Release law — CalVer, receipt chain, gate predicate |
| `Hook` | Hook graph wired and lawful |
| `Repair` | Repair plans present and applied |
| `Receipt` | Receipt chain valid and complete |
| `Security` | Security law — no credential leaks, no oracle injection |
| `Autopoiesis` | Self-replication law — no circular code generation |
| `Domain` | Domain-specific law (set by RulePackServer) |
| `Custom(String)` | Arbitrary domain axis with a string label |

### Bitmask layout

Bits 0–10 correspond to the variants in the order above (`Protocol=0`, `Type=1`, ...,
`Autopoiesis=10`, `Domain=11`). `Custom` axes do not have fixed bit positions and are carried
only in the `Vec<LawAxis>` fields.

### Invariants

1. **Unknown must never collapse into Admitted or Refused.** Unknown means "not traced" — it is
   a distinct epistemic state from knowing the axis is clean or violated.
2. `admitted_bits & refused_bits == 0` — no axis can be simultaneously admitted and refused.
3. `admitted_bits | refused_bits | unknown_bits` may not cover all 12 defined bits — axes not
   present in any set are implicitly unknown at the workspace level.

### `admits_release()` predicate

```
admits_release() = refused.is_empty()
                   AND (strict_mode == false OR unknown.is_empty())
```

In strict mode, Unknown blocks release. In non-strict mode, Unknown is tolerated — the agent
may proceed but must record the Unknown axes in its handoff artifact.

---

## Gate predicate Λ_CD

Defined in `src/gate.rs`.

### Formal conditions

The ANDON gate fires (exit 1) when **any** of the following are true:

1. The compositor is in `Uninitialized` typestate (no `initialize` received)
2. No receipt is present for the current session discipline
3. Any child server has active WASM4PM-* diagnostics (count > 0 for any code family)
4. Any child server has active GGEN-* diagnostics (count > 0 for any code family)
5. The ConformanceVector has `refused` axes (workspace-level)

### Exit codes

| Exit | Meaning |
|------|---------|
| `0` | Gate clear — Λ_CD predicate satisfied |
| `1` | ANDON active — one or more conditions above are true |

### CLI commands

```bash
lsp-max-cli gate check          # exit 0 or 1
lsp-max-cli gate list           # list blocking families + counts
lsp-max-cli gate list --json    # machine-readable format
```

### PreToolUse hook enforcement

When wired in `.claude/settings.json`, the gate check runs before every Bash, Edit, and Write
tool call. The hook command is `lsp-max-cli gate check`. Exit 1 causes the harness to return
`"decision": "block"` — the tool call is rejected without execution.

### Clearing procedure

1. `lsp-max-cli gate list` — identify blocking families
2. Fix the underlying violation (edit source, fix template binding, etc.)
3. Send `textDocument/didSave` for affected files — triggers rescan
4. Wait for `textDocument/publishDiagnostics` with empty `diagnostics[]`
5. `lsp-max-cli gate check` — confirm exit 0

---

## LSIF 0.6 vertex types

All 24 LSIF 0.6 vertex types. lsp-max extensions are marked **[ext]**.

| Label | Purpose |
|-------|---------|
| `metaData` | File header: version, projectRoot, toolInfo |
| `project` | Project vertex: kind (language), name |
| `document` | One source file: uri, languageId, contents? |
| `range` | A source range: start/end Position, tag? |
| `resultSet` | Groups results across rename/implementation chains |
| `hoverResult` | Hover text for a range |
| `definitionResult` | Locations of symbol definition |
| `declarationResult` | Locations of symbol declaration |
| `typeDefinitionResult` | Locations of type definition |
| `implementationResult` | Locations of interface implementations |
| `referencesResult` | All reference locations for a symbol |
| `documentSymbolResult` | All symbols in a document (outline) |
| `foldingRangeResult` | Folding ranges for a document |
| `documentLinkResult` | Hyperlinks within a document |
| `diagnosticResult` | Diagnostics for a document |
| `exportResult` | Exported monikers from a document |
| `importResult` | Imported monikers into a document |
| `packageInformation` | Package metadata (name, version, manager) |
| `moniker` | Symbol identifier across package boundaries (UniquenessLevel 0.6) |
| `$event` | Lifecycle events: begin/end for project and document vertices |
| `toolInfo` | Tool name, version, args — appears in metaData |
| `RangeTag` | 0.6 addition: typed range metadata (definition/reference/unknown) |
| `$lsp-max/conformanceNode` **[ext]** | Law-state node: admitted/refused/unknown LawAxis sets + score + receipt_id |
| `$lsp-max/diagnosticNode` **[ext]** | Aggregated diagnostic summary per document group |

### Key LSIF 0.6 additions over 0.4

- **Multi-in edges** (`inVs` array): one edge can link to multiple target vertices
- **Monikers with `UniquenessLevel`**: `document`, `project`, `group`, `scheme`, `global`
- **`RangeTag`**: distinguishes definition ranges from reference ranges
- **`ToolInfo`**: machine-readable tool metadata in metaData vertex
- **`$event`**: project-level and document-level lifecycle markers

---

## LSIF 0.6 edge types

All 21 LSIF 0.6 edge types. Multi-in capable edges are marked **[multi-in]**.

| Label | Direction | Multi-in | Purpose |
|-------|-----------|----------|---------|
| `contains` | project→document, document→range | no | Containment |
| `item` | resultSet→range **[multi-in]** | yes | Associates result set with ranges |
| `next` | range→resultSet | no | Links range to its result set |
| `textDocument/hover` | range→hoverResult | no | Hover result |
| `textDocument/definition` | range→definitionResult | no | Definition |
| `textDocument/declaration` | range→declarationResult | no | Declaration |
| `textDocument/typeDefinition` | range→typeDefinitionResult | no | Type definition |
| `textDocument/implementation` | range→implementationResult | no | Implementation |
| `textDocument/references` | range→referencesResult | no | References |
| `textDocument/documentSymbol` | document→documentSymbolResult | no | Document symbols |
| `textDocument/foldingRange` | document→foldingRangeResult | no | Folding ranges |
| `textDocument/documentLink` | document→documentLinkResult | no | Document links |
| `textDocument/diagnostic` | document→diagnosticResult **[multi-in]** | yes | Diagnostics |
| `moniker` | range→moniker | no | Symbol identity |
| `nextMoniker` | moniker→moniker | no | Moniker chain |
| `packageInformation` | moniker→packageInformation | no | Package |
| `$event` | project/document lifecycle | no | Lifecycle marker |
| `attach` | moniker→resultSet | no | Attaches moniker to result set |
| `belongsTo` | resultSet→resultSet | no | Hierarchy |
| `$lsp-max/conformanceEdge` **[ext]** | conformanceNode→document **[multi-in]** | yes | Links law-state to covered documents |
| `$lsp-max/receiptEdge` **[ext]** | conformanceNode→receiptVertex | no | Links node to receipt chain entry |

---

## ConformanceNode and ConformanceEdge schemas

lsp-max extensions to LSIF 0.6. Defined in `crates/lsp-max-lsif/src/`.

### ConformanceNode vertex

```jsonc
{
  "id": 147,
  "type": "vertex",
  "label": "$lsp-max/conformanceNode",
  "admitted": ["Type", "Protocol", "Receipt"],
  "refused": [],
  "unknown": ["Documentation"],
  "score": 0.875,
  "strict_mode": false,
  "receipt_id": "rcpt-9a2b-4f71-...",
  "timestamp": "2026-06-25T14:32:00Z"
}
```

### ConformanceEdge edge

Multi-in edge linking one `ConformanceNode` to multiple document vertices:

```jsonc
{
  "id": 148,
  "type": "edge",
  "label": "$lsp-max/conformanceEdge",
  "outV": 147,
  "inVs": [12, 34, 89],
  "document_uris": [
    "file:///workspace/crates/my-server/src/lib.rs",
    "file:///workspace/crates/my-server/Cargo.toml",
    "file:///workspace/src/rule_pack_server.rs"
  ]
}
```

The `inVs` array is the LSIF 0.6 multi-in pattern. Each entry is a document vertex id. The
`document_uris` array is a convenience field for consumers that do not want to resolve vertex ids.

---

## OCEL event schema

Defined in `crates/lsp-max-compositor/src/flush_coordinator.rs`.

### Activity types

| Activity | When emitted |
|----------|-------------|
| `CompositorFlush` | FlushCoordinator completes a flush cycle |
| `CompositorFlushAdmitted` | Flush cycle result: all Declare constraints satisfied |
| `CompositorFlushBlocked` | Flush cycle result: one or more Declare constraints violated |
| `AndonCodePresent` | At least one WASM4PM-* or GGEN-* diagnostic is active |
| `ForbiddenRefDetected` | anti-llm-cheat-lsp detected `tower_lsp` or `tower-lsp` |
| `FakeReceiptDetected` | anti-llm-cheat-lsp detected a fake receipt pattern |
| `VictoryLanguageDetected` | anti-llm-cheat-lsp detected victory language |
| `VersionViolationDetected` | anti-llm-cheat-lsp detected a CalVer violation |
| `ScanComplete` | Synthetic terminal activity — scan cycle complete |

### Event structure

```jsonc
{
  "ocel:type": "CompositorFlushAdmitted",
  "ocel:id": "evt-3a4b-...",
  "ocel:timestamp": "2026-06-25T14:32:01.123Z",
  "ocel:attributes": {
    "flush_id": "flush-001",
    "document_count": 3,
    "fitness": 0.875,
    "precision": 0.778
  },
  "ocel:relationships": [
    {
      "ocel:objectId": "doc-crates-my-server-lib-rs",
      "ocel:qualifier": "flushed_document"
    }
  ]
}
```

### OcelObject

```jsonc
{
  "ocel:type": "Document",
  "ocel:id": "doc-crates-my-server-lib-rs",
  "ocel:attributes": {
    "uri": "file:///workspace/crates/my-server/src/lib.rs",
    "language_id": "rust",
    "version": 3
  }
}
```

---

## Declare constraint types

Defined in `crates/lsp-max-compositor/src/declare.rs`. All 9 constraint types from the Van der
Aalst Declare specification, implemented as LTL-style checks over traces.

| Constraint | Formal semantics | English |
|-----------|-----------------|---------|
| `Init(A)` | First event in trace is A | Activity A must be the first in every case |
| `End(A)` | Last event in trace is A | Activity A must be the last in every case |
| `Response(A, B)` | After A, eventually B | If A occurs, B must occur after it |
| `Precedence(A, B)` | B occurs only after A | A must occur before B in every case |
| `ExactlyOne(A)` | A occurs exactly once | Activity A occurs once — no more, no less |
| `NotCoExistence(A, B)` | Not (A and B) in same trace | A and B are mutually exclusive per case |
| `RespondedExistence(A, B)` | If A occurs, B occurs (anywhere) | B must exist if A does |
| `Absence(A)` | A never occurs | Forbidden activity |
| `ChainResponse(A, B)` | After A, immediately B (no interleaving) | B must immediately follow A |

### Normative models

`DeclareModel::compositor()` — the 9-constraint normative model for the compositor flush pipeline.
Used by `lsp-max-cli process variants --model compositor`.

`DeclareModel::anti_llm_detection()` — normative model for the anti-llm detection pipeline.
Enforces that `ForbiddenRefDetected` is always followed by `ScanComplete`, and that `VictoryLanguageDetected`
is mutually exclusive with `CompositorFlushAdmitted`.

---

## Receipt chain

Defined in `crates/lsp-max-protocol/src/receipt.rs` and
`crates/lsp-max-compositor/src/receipt_chain.rs`.

### CryptographicReceipt fields

| Field | Type | Meaning |
|-------|------|---------|
| `prev_hash` | `[u8; 32]` | Blake3 hash of the previous receipt in the chain |
| `discipline_id` | `Uuid` | Identifies the law domain this receipt covers |
| `law_id` | `Uuid` | Identifies the specific law or constraint |
| `consequence_hash` | `[u8; 32]` | Blake3 hash of the evidence artifact (OCEL log, source content) |
| `sequence` | `u64` | Monotonically increasing sequence number within the discipline |
| `signature` | `[u8; 64]` | Ed25519 signature over `prev_hash || law_id || consequence_hash || sequence` |

### ChildEvidence

Links a receipt to the diagnostic that produced it:

| Field | Type | Meaning |
|-------|------|---------|
| `server_id` | `String` | ID of the child server that produced the finding |
| `receipt` | `CryptographicReceipt` | The receipt itself |
| `symbol_object_id` | `Option<String>` | Symbol in the workspace the receipt is bound to |
| `has_andon_contribution` | `bool` | Whether this receipt was produced during an active ANDON signal |

### Chain validation

```bash
scripts/validate-receipt-chain.sh receipts/rcpt-9a2b-....receipt.json
lsp-max-cli receipt validate --id rcpt-9a2b-...
```

Validation checks:
1. Boundary markers present (`-----BEGIN RECEIPT-----` / `-----END RECEIPT-----`)
2. SHA256/Blake3 digest matches content
3. Sequence numbers are monotonically increasing
4. `prev_hash` of receipt N matches hash of receipt N-1
5. Ed25519 signature verifiable with the server's public key

---

## Virtual document URIs

lsp-max serves synthetic documents at these URIs. Request via `textDocument/didOpen` with the URI;
content is served by the compositor without touching the filesystem.

| URI | Content | Refresh |
|-----|---------|---------|
| `anti-llm://process-model` | Live Mermaid DFG + Declare conformance report from active `AntiLlmDiagnostic` state | On every diagnostic change |
| `lsif://workspace` | Markdown summary: vertex count, ConformanceVector table, receipt status, DFG fitness | On every flush cycle |
| `max://conformance/{workspace}` | JSON ConformanceVector for the named workspace | On every `max/admission` change |
| `max://gate` | Gate state JSON: `{"gate": "CLEAR"|"BLOCKED", "families": [...]}` | On every diagnostic change |
| `max://snapshot/{ts}` | Point-in-time SnapshotBundle at timestamp `ts` | Static after creation |

### Accessing a virtual document

```json
{
  "jsonrpc": "2.0",
  "method": "textDocument/didOpen",
  "params": {
    "textDocument": {
      "uri": "anti-llm://process-model",
      "languageId": "markdown",
      "version": 1,
      "text": ""
    }
  }
}
```

The compositor responds with a `textDocument/publishDiagnostics` (empty) and serves the content
via hover or document symbol requests. Alternatively, use `lsp-max-cli` to fetch directly:

```bash
lsp-max-cli snapshot show --uri lsif://workspace
lsp-max-cli snapshot show --uri max://gate
```

---

## RulePackServer trait

Defined in `src/rule_pack_server.rs`. Re-exports from `lsp_max`.

### Five abstract methods

| Method | Return type | What to return |
|--------|-------------|---------------|
| `rule_packs(&self)` | `&ValidatedRulePackSet` | The loaded rule pack set (use `::empty()` for engine-bridge servers) |
| `grammar(&self)` | `tree_sitter::Language` | The tree-sitter grammar for the target language |
| `server_name(&self)` | `&'static str` | Human-readable server identifier (used in diagnostics `source` field) |
| `client(&self)` | `&Client` | The LSP client handle (for publishing diagnostics) |
| `adapter(&self)` | `&AutoLspAdapter` | The AST adapter from `lsp-max-adapters` |

### Optional overrides

| Method | Default | When to override |
|--------|---------|-----------------|
| `workspace_index(&self)` | `None` | Return `Some(&self.workspace_index)` to enable automatic upsert/remove |
| `scan_uri_classified(&self, uri, content)` | Runs rule pack engine | When the server has its own scanner (AhoCorasick, regex, AST walk) |
| `handle_did_open(...)` | Calls `scan_uri_classified` + publish | When additional logic is needed on open |
| `handle_did_change(...)` | Calls `scan_uri_classified` + publish | When additional logic is needed on change |

### Key types

```rust
// ClassifiedFindings = (sync_findings, background_findings)
// Sync findings published immediately; background after debounce
type ClassifiedFindings = (Vec<Finding>, Vec<Finding>);

// Finding = (MaxDiagnostic, lsp_types::Diagnostic)
type Finding = (MaxDiagnostic, Diagnostic);
```

### LawAxis variants for scan_uri_classified

Use `LawAxis::Domain` for server-specific law enforcement. Use `LawAxis::Custom(String)` when the
finding belongs to a named sub-domain (e.g., `LawAxis::Custom("ocel-lifecycle")`).

---

## CLI noun/verb grammar

The CLI is a noun/verb actuation grammar. Each noun module in
`crates/lsp-max-cli/src/nouns/` is one noun; `#[verb]`-annotated functions are actions.

### Key noun/verb combinations

| Noun | Verb | Effect |
|------|------|--------|
| `gate` | `check` | Exit 0 if clear, 1 if ANDON |
| `gate` | `list` | List blocking families + counts |
| `conformance` | `score` | Print ConformanceVector + admits_release() |
| `conformance` | `vector` | JSON ConformanceVector |
| `diagnostics` | `list` | All active diagnostics (optionally filtered by file) |
| `diagnostics` | `watch` | Stream diagnostics as they arrive (long-running) |
| `ocel` | `events` | Drain OCEL events since a time |
| `ocel` | `export` | Export OCEL 2.0 JSON to file |
| `process` | `dfg` | Build DFG from OCEL events and print fitness/precision |
| `process` | `variants` | Run Declare conformance against a normative model |
| `receipt` | `validate` | Validate receipt chain |
| `receipt` | `show` | Print receipt fields |
| `server` | `start` | Start a child server |
| `server` | `list` | List registered servers + status |
| `server` | `health` | Health check for one server |
| `snapshot` | `show` | Show virtual document content by URI |
| `export` | `lsif` | Export LSIF 0.6 NDJSON to file |
| `admission` | `status` | Query admission status per axis |

### Global flags

| Flag | Meaning |
|------|---------|
| `--json` | Machine-readable JSON output |
| `--workspace <path>` | Override workspace root (default: cwd) |
| `--state <path>` | Override `$LSP_MAX_STATE_PATH` |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | ADMITTED / gate clear / success |
| `1` | BLOCKED / gate active / error |
| `2` | UNKNOWN / not enough information |
| `3` | REFUSED / explicit refusal recorded |

---

## Haiku model policy

`claude-haiku-4-5-20251001` is permitted only for read-only roles. It must not produce file
mutations, shell commands that alter state, or any output that becomes part of the production
codebase.

| Role | Haiku allowed? | Notes |
|------|---------------|-------|
| Read-only research, summarization | Yes | No file writes |
| Code review, diagnostic analysis | Yes | No file writes |
| Documentation drafting (not committed) | Yes | Output goes to human review only |
| Editing source files (Edit/Write tool) | No | Use claude-sonnet-4-6 |
| Bash commands that alter filesystem | No | Use claude-sonnet-4-6 |
| Commit authoring | No | Use claude-sonnet-4-6 |
| Gate-clearing repairs | No | Use claude-sonnet-4-6 |
| Agent preamble gate check | Yes | Read-only operation |

Writing agents must use `claude-sonnet-4-6`. This policy is enforced by the agent configuration
in `AGENTS.md` and is a hard requirement, not a preference.
