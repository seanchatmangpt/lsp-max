# Agent how-to guides

Standalone recipes for common lsp-max integration tasks. Each recipe is self-contained. All
examples are framed as agent actions.

---

## Recipe 1 — Block agent execution on ANDON gate

Use this pattern to ensure an agent never acts while law-state is violated.

### Gate check preamble (every agent, every session)

```bash
lsp-max-cli gate check || { echo "ANDON gate is BLOCKED"; exit 1; }
```

Exit codes:
- `0` — gate clear, agent may proceed
- `1` — ANDON active, agent must stop and resolve

### Inspect blocking families

```bash
lsp-max-cli gate list
```

Returns the diagnostic code families (e.g., `WASM4PM-CHEAT-C001`, `GGEN-TPL-001`), scope (file
or workspace), and count per family.

### Wire the PreToolUse hook in `.claude/settings.json`

The hook enforces the gate before every Bash, Edit, and Write tool call made by the agent:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "lsp-max-cli gate check"
          }
        ]
      }
    ]
  }
}
```

When `gate check` exits 1, the hook returns `"decision": "block"` and the tool call is
rejected before execution. The agent receives a `PermissionDenied`-equivalent response and
must surface the gate state to the user or loop on resolution.

### Clearing WASM4PM-* diagnostics

WASM4PM-* codes come from `wasm4pm-lsp`. Each code family has a specific resolution:

| Code | What it means | How to clear |
|------|--------------|--------------|
| `WASM4PM-CHEAT-C001` | Seeded RNG / determinism oracle | Remove `rand::SeedableRng` or hardcoded seed |
| `WASM4PM-CHEAT-C002` | Hardcoded output metrics | Replace with measured values from OCEL replay |
| `WASM4PM-TS-A001` | Wrong ContractResult fields | Use `admitted`/`refused`/`unknown` field names |
| `WASM4PM-TS-FM5` | `vi.mock("init.js")` in tests | Load real WASM via `await init()` |
| `WASM4PM-CROWN-001` | Receipt missing `output_hash` | Add `output_hash` to `.receipt.json` |
| `WASM4PM-GALL-001` | OCEL Gall conformance deviation | Verify OCEL events match declared lifecycle |

After fixing, send `textDocument/didSave` to trigger rescan:

```json
{
  "jsonrpc": "2.0",
  "method": "textDocument/didSave",
  "params": { "textDocument": { "uri": "file:///workspace/src/my_breed.rs" } }
}
```

### Clearing GGEN-* diagnostics

GGEN-* codes come from `ggen-lsp`. They signal σ violations in the generation chain.

**Do NOT call `ggen sync` while any GGEN-* diagnostic is active.** Running ggen over a broken
template/query binding produces silently incorrect source.

| Code | σ violation | Resolution |
|------|------------|------------|
| `GGEN-TPL-001` | Template variable ≠ SPARQL SELECT output | Align `{{ variable }}` in `.tera` with SELECT columns in `.rq` |
| `GGEN-YIELD-001` | Output in pack root, not consumer root | Change `output_file` in `ggen.toml` to consumer-rooted path |
| `GGEN-YIELD-003` | Rendered source has no use-site | Add `mod` declaration or delete the yield entry |
| `GGEN-SRC-001` | `generated/` in output path | Rename to a non-generated path; rendered source is first-class |

---

## Recipe 2 — Build a RulePackServer for domain law

`RulePackServer` is the bridge trait for domain-specific diagnostic LSP servers. Five abstract
methods; everything else is defaulted.

### Minimal implementation

```rust
use lsp_max::{
    Client, ClassifiedFindings, Finding, ValidatedRulePackSet, WorkspaceIndex,
    RulePackServer,
};
use lsp_max::max_protocol::{MaxDiagnostic, LawAxis};
use lsp_max_adapters::AutoLspAdapter;
use lsp_types::{Diagnostic, DiagnosticSeverity, DocumentUri, Range, Position};

pub struct MyDomainServer {
    client: Client,
    rule_packs: ValidatedRulePackSet,
    ast_adapter: AutoLspAdapter,
    workspace_index: WorkspaceIndex,
}

impl RulePackServer for MyDomainServer {
    fn rule_packs(&self) -> &ValidatedRulePackSet { &self.rule_packs }
    fn grammar(&self) -> tree_sitter::Language { tree_sitter_rust::LANGUAGE.into() }
    fn server_name(&self) -> &'static str { "my-domain-lsp" }
    fn client(&self) -> &Client { &self.client }
    fn adapter(&self) -> &AutoLspAdapter { &self.ast_adapter }
    fn workspace_index(&self) -> Option<&WorkspaceIndex> { Some(&self.workspace_index) }
}
```

The defaults handle `did_open`, `did_change`, `did_close`, AST parsing, and
`publish_findings_classified → scan_uri_classified` pipeline.

### Override `scan_uri_classified` for an engine-bridge server

When the server has its own scanner (AhoCorasick, regex, custom AST walk), override this method
to bridge it into `ClassifiedFindings`:

```rust
fn scan_uri_classified(&self, uri: &DocumentUri, content: &str) -> ClassifiedFindings {
    let raw_findings = self.my_engine.scan(content);

    let findings: Vec<Finding> = raw_findings
        .into_iter()
        .map(|hit| {
            let lsp_diag = Diagnostic {
                range: Range {
                    start: Position { line: hit.line, character: hit.col_start },
                    end: Position { line: hit.line, character: hit.col_end },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(lsp_types::NumberOrString::String(hit.code.clone())),
                source: Some("my-domain-lsp".to_string()),
                message: hit.message.clone(),
                ..Default::default()
            };
            let max_diag = MaxDiagnostic {
                lsp: lsp_diag.clone(),
                law_axis: LawAxis::Domain,
                ..MaxDiagnostic::default()
            };
            (max_diag, lsp_diag)
        })
        .collect();

    // Return (sync_findings, background_findings)
    // Sync findings are published immediately; background findings after a debounce.
    (findings, vec![])
}
```

### Wire WorkspaceIndex

`WorkspaceIndex` is `Arc<DashMap<String, IndexedDoc>>`. The default `handle_did_*` methods call
`upsert` and `remove` automatically when `workspace_index()` returns `Some`.

```rust
use lsp_max::WorkspaceIndex;
use std::sync::Arc;
use dashmap::DashMap;

// In server constructor:
let workspace_index: WorkspaceIndex = Arc::new(DashMap::new());
```

### Canonical example

`crates/anti-llm-cheat-lsp/src/` is the reference implementation. Key files:

- `server.rs` — `AntiLlmCheatServer` struct, `RulePackServer` impl with `scan_uri_classified` override
- `engine.rs` — AhoCorasick pattern scanner bridged into `ClassifiedFindings`
- `diagnostics.rs` — `AntiLlmDiagnostic` type, code-to-message mappings

The virtual document `anti-llm://process-model` in `virtual_docs.rs` shows how to surface a live
DFG + Declare conformance report from active diagnostic state.

---

## Recipe 3 — Consume LSIF 0.6 without a running server

LSIF 0.6 exports let CI/CD gates and handoff agents query law-state without a live LSP session.

### Request the LSIF stream

Via `max/lsif` (if a server is running):

```json
{
  "jsonrpc": "2.0",
  "method": "max/lsif",
  "id": 1,
  "params": {
    "projectRoot": "file:///workspace",
    "includeConformanceNodes": true,
    "outputFormat": "ndjson"
  }
}
```

Or from a pre-exported file (CI/CD, agent handoff):

```bash
lsp-max-cli export lsif --output /tmp/workspace.lsif.ndjson
```

### Parse ConformanceNode vertices in a CI script

```bash
#!/usr/bin/env bash
# Extract ConformanceNode vertices and check for refused axes

LSIF_FILE="${1:?usage: check-conformance.sh <lsif.ndjson>}"

# Find all ConformanceNode vertices
REFUSED=$(jq -r 'select(.label == "$lsp-max/conformanceNode") | .refused[]' "$LSIF_FILE" | sort -u)

if [[ -n "$REFUSED" ]]; then
  echo "REFUSED axes found:"
  echo "$REFUSED"
  echo "Gate: REFUSED — release blocked"
  exit 1
fi

SCORE=$(jq -r 'select(.label == "$lsp-max/conformanceNode") | .score' "$LSIF_FILE" | tail -1)
RECEIPT=$(jq -r 'select(.label == "$lsp-max/conformanceNode") | .receipt_id' "$LSIF_FILE" | tail -1)

echo "Score: $SCORE"
echo "Receipt: $RECEIPT"
echo "Gate: ADMITTED"
exit 0
```

### Trace ConformanceEdge to documents

`ConformanceEdge` uses multi-in edges (LSIF 0.6 `inVs` array) to link one `ConformanceNode` to
multiple document vertices:

```bash
# Find which documents a ConformanceNode covers
NODE_ID=147
jq --argjson node_id "$NODE_ID" \
  'select(.label == "$lsp-max/conformanceEdge" and .outV == $node_id) | .inVs' \
  workspace.lsif.ndjson
```

### Receipt traceability

The `receipt_id` field in `ConformanceNode` links to the `CryptographicReceipt` chain. Validate:

```bash
lsp-max-cli receipt validate --id rcpt-9a2b-...
# or
scripts/validate-receipt-chain.sh receipts/rcpt-9a2b-....receipt.json
```

---

## Recipe 4 — Emit OCEL events and run Van der Aalst conformance

### Drain accumulated events from the compositor

```bash
lsp-max-cli ocel events --since session-start --format json > /tmp/session.ocel.json
```

Or via protocol:

```json
{
  "jsonrpc": "2.0",
  "method": "max/autonomicLoop",
  "id": 1,
  "params": { "action": "drain_ocel" }
}
```

`FlushCoordinator::take_ocel_events()` drains the internal buffer. Each event is an OCEL 2.0
object with `activity`, `timestamp`, `object_id`, and optional `relationships`.

### Run Declare conformance

```bash
lsp-max-cli process variants --model compositor
```

This runs `DeclareModel::compositor().check(&traces)` against the drained events. The compositor
normative model has 9 constraint types. A zero-violation result means the event log matches the
declared flush pipeline.

To run against the anti-llm detection model:

```bash
lsp-max-cli process variants --model anti-llm-detection
```

### Build a DFG and compute fitness

```bash
lsp-max-cli process dfg --events /tmp/session.ocel.json --normative compositor
```

Output:

```
DFG nodes: 7
DFG edges: 9
Normative arcs: 8

Fitness:    0.875  (7/8 normative arcs present)
Precision:  0.778  (7/9 DFG arcs in normative model)

Missing arc: ForbiddenRefDetected → RepairApplied
Excess arc:  ScanComplete → ForbiddenRefDetected  (loop — possible rework)
```

A missing arc means the expected transition did not occur in this session. An excess arc means an
unexpected transition occurred — possibly rework or a retry loop that the declared model does not
account for.

### In Rust (programmatic)

```rust
use lsp_max_compositor::{
    dfg::DirectlyFollowsGraph,
    declare::DeclareModel,
};

let events = flush_coordinator.take_ocel_events();
let traces = extract_traces(&events);

// Declare conformance
let model = DeclareModel::compositor();
let violations = model.check(&traces);
assert!(violations.is_empty(), "Declare violations: {:?}", violations);

// DFG fitness
let dfg = DirectlyFollowsGraph::from_traces(&traces);
let normative_arcs = DeclareModel::compositor_arcs();
let fitness = dfg.fitness_against_model(&normative_arcs);
assert!(fitness.unwrap_or(0.0) > 0.8, "DFG fitness too low: {:?}", fitness);
```

---

## Recipe 5 — Query ConformanceVector in an agent decision loop

### Never collapse Unknown

The `ConformanceVector` has three axis sets: `admitted`, `refused`, `unknown`. Unknown must never
be collapsed into either polarity — it means "not yet traced," which is different from "admitted"
or "refused."

```rust
let cv: ConformanceVector = compositor.request(max_admission_request).await?;

// CORRECT
if !cv.refused.is_empty() {
    agent.block_release("refused axes present");
}
if cv.admitted_release() {
    agent.proceed_to_release();
}
// unknown axes: do not proceed to ADMITTED, but do not fail unless strict_mode

// WRONG — do not do this
let all_admitted = cv.unknown.is_empty() && cv.refused.is_empty(); // ignores Unknown meaning
```

### `admits_release()` semantics

```
admits_release() = refused.is_empty()
                   AND (strict_mode == false OR unknown.is_empty())
```

When `strict_mode = true`, Unknown axes block release. This is appropriate for production gates.
When `strict_mode = false`, Unknown axes are tolerated — appropriate for development sessions
where not all axes have been traced yet.

### Poll ConformanceVector during an agent repair loop

```bash
# Agent repair loop: keep fixing until admits_release() is true
while true; do
  SCORE=$(lsp-max-cli conformance score --workspace --json | jq '.admits_release')
  if [ "$SCORE" = "true" ]; then
    echo "ConformanceVector: ADMITTED for release"
    break
  fi
  # Agent applies next repair, then loops
  sleep 1
done
```

### Via max/workspaceConformance

```json
{
  "jsonrpc": "2.0",
  "method": "max/workspaceConformance",
  "id": 1,
  "params": { "strict_mode": false }
}
```

Returns a `ConformanceVector` over the entire workspace — merged from all child server vectors
via the compositor's merge policy (union of admitted, union of refused, union of unknown across
all documents).

---

## Recipe 6 — Use the MCP bridge for MCP-native agents

`crates/lsp-max-mcp/` exposes lsp-max capabilities as MCP tools. MCP-native agents (those that
speak MCP tool-call protocol) can use these without implementing the LSP wire protocol.

### Available MCP tools

| Tool | Purpose |
|------|---------|
| `lsp_discover` | List all registered child LSP servers and their capabilities |
| `lsp_route` | Route an LSP request to a specific child server |
| `lsp_health` | Check compositor health and gate state |
| `lsp_reload_config` | Reload `lsp-max.toml` without restarting the compositor |
| `lsp_gate_check` | Gate predicate query — returns `CLEAR` or `BLOCKED` with diagnostic families |

### Example: MCP agent checks gate before acting

```json
{
  "tool": "lsp_gate_check",
  "input": {}
}
```

Response:

```json
{
  "gate": "CLEAR",
  "active_families": [],
  "admits_release": true
}
```

### Why the LSP server must still run

MCP is pull: the agent asks, the server answers. It cannot receive push notifications. The
ambient law-enforcement model — where the server detects a violation the moment a file is saved —
requires LSP's push model (`textDocument/publishDiagnostics`).

The MCP bridge is appropriate when:
- An agent is MCP-native and cannot speak LSP directly
- The agent needs to query state at a specific point (e.g., before a commit)
- The agent is composing with other MCP tools and wants a uniform interface

The MCP bridge is insufficient when:
- The agent must be interrupted immediately on violation (use LSP push)
- The agent needs continuous ambient monitoring (use LSP push)
- The agent needs to subscribe to ConformanceVector changes (use LSP push)

For agents that can speak both protocols, use MCP for point queries and LSP for ambient monitoring.

---

## Recipe 7 — Wire a new child server into the compositor

The compositor routes LSP messages to child servers based on `lsp-max.toml` configuration.

### Add an entry to `lsp-max.toml`

```toml
[[servers]]
id = "my-domain-lsp"
command = "my-domain-lsp"
args = []
routing_tier = "Primary"

[servers.extensionToLanguage]
rs = "rust"
toml = "toml"
```

### Routing tiers

| Tier | What it means |
|------|--------------|
| `Primary` | Receives all LSP methods: `hover`, `completion`, `definition`, `references`, `diagnostics`, `didOpen/Change/Close` |
| `DiagnosticsOnly` | Receives only `didOpen`, `didChange`, `didClose`, `didSave` — used for law-enforcement servers that scan but do not provide code intelligence |

Use `DiagnosticsOnly` for servers like `anti-llm-cheat-lsp` and `wasm4pm-lsp` that scan for
violations but do not handle hover or completion requests. Use `Primary` for full-featured LSP
servers (rust-analyzer, typescript-language-server).

### Verify routing

After reloading config:

```bash
lsp-max-cli server list
```

Expected output:

```
Registered servers:
  compositor          Primary     ws://127.0.0.1:2087   ADMITTED
  anti-llm-cheat-lsp DiagnosticsOnly  stdio             ADMITTED
  my-domain-lsp      Primary          stdio             CANDIDATE
```

`CANDIDATE` means the server registered successfully but its first diagnostic sweep has not
completed. After the first `textDocument/didOpen`, it transitions to `ADMITTED` or surfaces its
first diagnostics.

### Merge policy for ConformanceVector

When multiple child servers return `ConformanceVector` results, the compositor merges them:

```
merged.admitted = INTERSECTION of admitted sets (axis admitted only if ALL children admit it)
merged.refused  = UNION of refused sets (axis refused if ANY child refuses it)
merged.unknown  = UNION of unknown sets minus admitted and refused
```

This is conservative: one child refusing an axis blocks the workspace-level admission for that
axis, and one child not having traced an axis keeps it Unknown until all children have seen it.
