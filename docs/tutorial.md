# Tutorial: Build an agent loop that self-corrects from LSP law signals

This tutorial walks through a complete agent loop that uses lsp-max to enforce law-state at
runtime. By the end, an agent will: detect a violation through LSP push diagnostics, request a
repair plan, apply a fix, drain OCEL events to confirm a lawful process ran, and export LSIF 0.6
to prove admission — all without human intervention.

**Audience:** Agent developers wiring lsp-max into a coding agent. All steps are framed as agent
actions, not human steps.

**Prerequisites:**
- `lsp-max-cli` is in PATH (`cargo install --path crates/lsp-max-cli`)
- Sibling repos present: `../lsp-types-max`, `../wasm4pm-compat`, `../wasm4pm`
- Compositor running at `ws://127.0.0.1:2087` (or via stdio)

---

## Step 1 — Agent preamble: gate check before any action

Every agent preamble **must** begin with a gate check. The ANDON gate blocks all Bash, Edit, and
Write tool calls when active WASM4PM-* or GGEN-* diagnostics are present. The gate check exits 0
(clear) or 1 (ANDON active).

```bash
lsp-max-cli gate check || { echo "ANDON gate is BLOCKED — resolve diagnostics before proceeding"; exit 1; }
```

To inspect which diagnostic families are blocking:

```bash
lsp-max-cli gate list
```

Sample output when blocked:

```
BLOCKED families:
  WASM4PM-CHEAT-C001  scope: workspace  count: 2
  GGEN-TPL-001        scope: crates/my-server  count: 1

Gate predicate Λ_CD: REFUSED
```

The agent must not proceed until `gate check` exits 0. This is not a suggestion — the PreToolUse
hook enforces it by returning `"decision": "block"` from `.claude/settings.json`.

---

## Step 2 — Start the compositor with anti-llm-cheat-lsp wired

The compositor fans LSP messages to all registered child servers and merges their diagnostics.
`anti-llm-cheat-lsp` is the default law-enforcement child.

Start the compositor (agent action via lsp-max-cli):

```bash
lsp-max-cli server start --id compositor --port 2087 --child anti-llm-cheat-lsp
```

Or, if the agent is driving a subprocess directly:

```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "id": 1,
  "params": {
    "capabilities": {
      "textDocument": {
        "publishDiagnostics": { "relatedInformation": true }
      }
    },
    "rootUri": "file:///workspace"
  }
}
```

The compositor responds with its capability set, including the `max/*` protocol extensions the
agent will use in later steps.

Verify the compositor is healthy:

```bash
lsp-max-cli server health --id compositor
```

Expected response: `{"status": "ADMITTED", "child_count": 1, "gate": "CLEAR"}`

---

## Step 3 — Agent opens a Rust file and sends `textDocument/didOpen`

The agent is about to edit `crates/my-server/src/lib.rs`. Before making any changes, it notifies
the compositor that the file is now in the agent's working set.

```json
{
  "jsonrpc": "2.0",
  "method": "textDocument/didOpen",
  "params": {
    "textDocument": {
      "uri": "file:///workspace/crates/my-server/src/lib.rs",
      "languageId": "rust",
      "version": 1,
      "text": "use tower_lsp::LspService;\n// rest of file...\n"
    }
  }
}
```

The compositor receives this notification and fans it to all child servers. `anti-llm-cheat-lsp`
receives the document content, runs its AhoCorasick scanner, and immediately detects the forbidden
reference `tower_lsp`.

**No agent polling required.** The push arrives within milliseconds.

---

## Step 4 — Violation detected — compositor pushes diagnostic

The compositor receives the child server's `ClassifiedFindings` and publishes them to the agent
via `textDocument/publishDiagnostics`:

```json
{
  "jsonrpc": "2.0",
  "method": "textDocument/publishDiagnostics",
  "params": {
    "uri": "file:///workspace/crates/my-server/src/lib.rs",
    "diagnostics": [
      {
        "range": {
          "start": { "line": 0, "character": 4 },
          "end": { "line": 0, "character": 14 }
        },
        "severity": 1,
        "code": "ANTI-LLM-CHEAT-LSP-001",
        "source": "anti-llm-cheat-lsp",
        "message": "Forbidden reference: plain 'tower_lsp' — use 'lsp_max' everywhere outside negative-control fixtures"
      }
    ]
  }
}
```

The agent reads this notification. The diagnostic code `ANTI-LLM-CHEAT-LSP-001` signals a
`ForbiddenRefDetected` activity in the process model. The agent must not attempt a repair
without first querying the repair plan — guessing the fix risks introducing a second violation.

The agent can also confirm ANDON is now active:

```bash
lsp-max-cli gate check
# exit 1 — ANDON active
lsp-max-cli diagnostics list --file crates/my-server/src/lib.rs
```

---

## Step 5 — Agent calls `max/repairPlan` to get an actionable repair

The `max/repairPlan` method returns a structured repair plan for a given diagnostic. The agent
sends this request to the compositor:

```json
{
  "jsonrpc": "2.0",
  "method": "max/repairPlan",
  "id": 2,
  "params": {
    "uri": "file:///workspace/crates/my-server/src/lib.rs",
    "diagnosticCode": "ANTI-LLM-CHEAT-LSP-001",
    "range": {
      "start": { "line": 0, "character": 4 },
      "end": { "line": 0, "character": 14 }
    }
  }
}
```

The compositor routes to `anti-llm-cheat-lsp` which returns a `RepairPlan`:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "plan_id": "plan-7f3a-...",
    "description": "Replace forbidden 'tower_lsp' reference with canonical 'lsp_max'",
    "transactions": [
      {
        "uri": "file:///workspace/crates/my-server/src/lib.rs",
        "edits": [
          {
            "range": {
              "start": { "line": 0, "character": 4 },
              "end": { "line": 0, "character": 14 }
            },
            "newText": "lsp_max"
          }
        ]
      },
      {
        "uri": "file:///workspace/crates/my-server/Cargo.toml",
        "edits": [
          {
            "range": {
              "start": { "line": 8, "character": 0 },
              "end": { "line": 8, "character": 29 }
            },
            "newText": "lsp-max = { path = \"../../\" }"
          }
        ]
      }
    ],
    "law_axis": "Type",
    "confidence": "HIGH",
    "requires_rebuild": true
  }
}
```

The agent applies both transactions in sequence — source edit first, manifest second — so the
workspace stays consistent.

---

## Step 6 — Agent applies the fix and sends `max/applyRepairTransaction`

Rather than applying workspace edits directly, the agent routes the repair through the compositor
so it can witness the mutation:

```json
{
  "jsonrpc": "2.0",
  "method": "max/applyRepairTransaction",
  "id": 3,
  "params": {
    "plan_id": "plan-7f3a-...",
    "transaction_index": 0
  }
}
```

The compositor applies the edit, then sends a `textDocument/didChange` notification to all child
servers with the updated content. `anti-llm-cheat-lsp` rescans — the forbidden reference is gone.
The diagnostic is cleared.

After applying both transactions, the agent confirms gate state:

```bash
lsp-max-cli gate check
# exit 0 — gate clear
```

The ANDON signal is released. The agent may now proceed with further edits or build steps.

---

## Step 7 — Agent drains OCEL events and confirms lawful process

The agent requests the accumulated OCEL 2.0 event log from the compositor to verify that the
expected process ran — not just that the code looks correct.

```bash
lsp-max-cli ocel events --since session-start
```

Or via the `max/*` protocol:

```json
{
  "jsonrpc": "2.0",
  "method": "max/autonomicLoop",
  "id": 4,
  "params": { "action": "drain_ocel" }
}
```

The returned events must contain the full repair arc. The agent runs Van der Aalst conformance
against the normative `DeclareModel::compositor()` model:

```bash
lsp-max-cli process variants --model compositor
```

Expected output (all constraints satisfied):

```
Traces: 1
Constraints checked: 9
Violations: 0
Fitness: 1.000
Precision: 0.871

Declare conformance: ADMITTED
```

A `Violations: 0` result means the event log is consistent with the declared process model —
`ForbiddenRefDetected` was followed by `ScanComplete`, with no impossible transitions.

If `Violations > 0`, the agent must not proceed to release. The OCEL log is the authoritative
record; code appearance is insufficient evidence.

---

## Step 8 — Agent calls `max/lsif` and finds ConformanceNode showing admission

The final step exports LSIF 0.6 NDJSON. This is the artifact that proves admission without
requiring a live server — CI gates and handoff agents can query it offline.

```json
{
  "jsonrpc": "2.0",
  "method": "max/lsif",
  "id": 5,
  "params": {
    "projectRoot": "file:///workspace",
    "includeConformanceNodes": true
  }
}
```

The compositor streams NDJSON. The agent parses the stream and locates `ConformanceNode` vertices:

```jsonl
{"id":1,"type":"vertex","label":"metaData","version":"0.6.0","projectRoot":"file:///workspace","toolInfo":{"name":"lsp-max","version":"26.6.21"}}
{"id":2,"type":"vertex","label":"project","kind":"rust"}
...
{"id":147,"type":"vertex","label":"$lsp-max/conformanceNode","admitted":["Type","Protocol","Receipt"],"refused":[],"unknown":["Documentation"],"score":0.875,"strict_mode":false,"receipt_id":"rcpt-9a2b-..."}
{"id":148,"type":"edge","label":"$lsp-max/conformanceEdge","outV":147,"inVs":[12,34,89],"document_uris":["file:///workspace/crates/my-server/src/lib.rs","file:///workspace/crates/my-server/Cargo.toml","file:///workspace/src/rule_pack_server.rs"]}
```

The `admitted` array contains `"Type"` — the axis that was violated and then repaired. `refused`
is empty. `unknown` contains `"Documentation"` — this axis has not been traced, but the
`strict_mode: false` flag means it does not block admission.

The agent calls `admits_release()` logic:

```bash
lsp-max-cli conformance score --workspace
```

```
ConformanceVector:
  admitted: Type, Protocol, Receipt
  refused:  (none)
  unknown:  Documentation
  score:    0.875
  strict_mode: false

admits_release(): true
```

The loop is COMPLETE. Law-state evidence exists: a receipt chain, a conforming OCEL event log,
and an LSIF 0.6 export with a `ConformanceNode` showing admission. The agent may proceed to
release actuation via `max/releaseActuation`.

---

## What this tutorial demonstrated

| Step | LSP mechanism | Law-state surface |
|------|--------------|-------------------|
| Gate preamble | CLI gate check | Λ_CD predicate |
| didOpen | Push notification fan-out | Compositor routing |
| Violation detection | publishDiagnostics push | ANTI-LLM-CHEAT-LSP-001 |
| Repair query | max/repairPlan | RepairPlan with transactions |
| Fix application | max/applyRepairTransaction | Witnessed mutation |
| Process verification | ocel events + process variants | Declare conformance |
| Admission export | max/lsif | ConformanceNode (LSIF 0.6) |

The key insight: the agent never had to ask "is the code lawful?" — the server pushed the answer
the moment the file changed. The repair plan was structured data, not a prose suggestion. The
OCEL log was the proof, not the agent's memory of what it did.
