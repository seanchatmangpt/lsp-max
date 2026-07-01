# Why LSP is the ambient law-state substrate for coding agents

This document explains the architectural decisions behind lsp-max: why LSP was chosen over MCP
or A2A as the law-enforcement substrate, what "law-state runtime" means beyond code intelligence,
how LSIF 0.6 bridges live sessions and stateless CI/CD, why the three-valued logic of
`ConformanceVector` is essential, how OCEL process mining closes the gap between "code looks
correct" and "lawful process ran," and how the ggen + RulePackServer pattern scales domain law
enforcement to new surfaces.

---

## The MCP/A2A gap: why request-response is insufficient for ambient law enforcement

MCP (Model Context Protocol) and A2A (Agent-to-Agent) solve real problems. MCP gives agents a
uniform interface to tools and context: the agent asks, the server answers. A2A gives agents a
coordination substrate: one agent delegates to another, with structured handoffs.

Neither protocol is designed for ambient law enforcement.

**MCP is pull.** The agent initiates every exchange. The server cannot interrupt the agent to say
"the file you just edited violates law X." The agent must explicitly ask — and it must know to
ask. In a coding agent loop, the agent is focused on its current task: generating a fix, applying
an edit, running a test. The agent does not know it should poll for law violations between every
edit. If it did poll, it would need to know which server to ask, which endpoints to call, and
how to interpret the response. The protocol provides no ambient channel for law signals.

**A2A is coordination.** It handles "which agent does what" — task delegation, result handoff,
capability negotiation. It does not handle "is the work the agent is doing lawful right now." A2A
assumes the agents themselves are operating lawfully; it does not provide a substrate for
enforcing that assumption.

**LSP is push.** The server observes every file mutation — `textDocument/didOpen`,
`textDocument/didChange`, `textDocument/didSave` — and immediately pushes diagnostics without
the agent asking. The server has continuous workspace awareness. The agent receives law signals
the moment a violation is introduced, not on the next polling cycle.

This is the gap: MCP and A2A require the agent to already know something is wrong before it can
ask about it. LSP tells the agent something is wrong before the agent has a chance to proceed.

The distinction matters operationally. An agent that introduces a violation and proceeds to build,
test, and commit before discovering the violation has wasted work and created a longer repair
cycle. An agent operating under LSP law-state discovers the violation at edit time and repairs it
before any downstream action. The feedback loop is compressed from minutes (build → test → gate)
to milliseconds (edit → push diagnostic → repair plan).

---

## LSP as ambient witness: textDocument/didChange, push model, workspace model

The LSP push model has three properties that make it suitable as an ambient witness:

**File-mutation coverage.** Every edit the agent makes passes through `textDocument/didChange`.
The compositor fans this notification to all child servers. Each child server rescans the document
independently. No edit escapes observation. The agent cannot accidentally bypass the law-state
check by using a file-write shortcut — if the file changes, LSP sees it.

**Push without subscription.** The server pushes diagnostics via `textDocument/publishDiagnostics`
without the agent subscribing to a specific event type. The agent receives law signals in its
notification channel alongside hover results and completion items. The agent does not need a
separate law-enforcement loop; law signals arrive in the existing LSP message flow.

**Workspace model.** LSP maintains a model of the entire workspace, not just the current file.
When a symbol is renamed, all files that reference the symbol receive updated diagnostics. When
a Cargo.toml changes, the server can re-evaluate law axes that depend on dependency declarations.
The law-state is computed over the workspace as a whole, not per-file in isolation.

lsp-max adds three layers to standard LSP:
- **max/* protocol surface**: methods for ConformanceVector queries, repair plans, OCEL drain,
  LSIF export, and gate predicate evaluation
- **Fan-out compositor**: routes each LSP notification to all registered child servers in
  parallel; merges their `ClassifiedFindings` before publishing
- **Law-state runtime**: ConformanceVector per document and workspace, receipt chain, gate
  predicate Λ_CD, typestate machine for the server lifecycle

These layers turn LSP from a code-intelligence protocol into a law-state protocol.

---

## Law-state runtime vs code intelligence: what's new in lsp-max

Standard LSP servers provide code intelligence: hover, completion, definition, references,
diagnostics. Diagnostics in standard LSP are advisory — the editor highlights them; the developer
decides whether to fix them. There is no enforcement mechanism.

lsp-max adds enforcement:

**Gate predicate Λ_CD.** The ANDON gate fires when law-state is violated. A PreToolUse hook in
`.claude/settings.json` evaluates `lsp-max-cli gate check` before every Bash, Edit, and Write
tool call. When the gate fires, the tool call is blocked. The agent cannot proceed until the
violation is resolved. This is not a suggestion surface — it is a hard block.

**Receipt chain.** Claims of admission require receipt artifacts — cryptographic records with a
Blake3 hash chain and Ed25519 signatures. A test log message saying "all tests passed" is not a
receipt. A `CompositorFlushAdmitted` OCEL event attached to a `CryptographicReceipt` with a
validated hash chain is a receipt. The distinction prevents agents from reporting admission based
on appearance rather than evidence.

**ConformanceVector.** Law-state is multi-dimensional. An artifact can have `Protocol` axis
admitted (LSP protocol compliance proven), `Type` axis admitted (no forbidden type intermediaries),
and `Documentation` axis Unknown (not yet traced). The vector captures this without collapsing
dimensions. Standard LSP diagnostics have no comparable structure — they are a flat list of
messages with severity levels.

**Typestate machine.** The server lifecycle is governed by a typestate machine in
`crates/lsp-max-runtime/`. Transitions require evidence: `initialize → ready` requires a valid
`initialize` response; `ready → admitted` requires a receipt. Illegal transitions are rejected at
compile time (Rust typestate pattern) and at runtime (gate predicate check).

---

## LSIF 0.6 as the stateless bridge: how agents get intelligence without a running session

The fundamental problem with live LSP sessions in agent workflows is that sessions are ephemeral.
A coding agent runs, makes edits, and terminates. The next agent in the pipeline — or the CI/CD
gate — has no access to the previous agent's LSP session. Law-state accumulated during the
editing session evaporates.

LSIF 0.6 (Language Server Index Format) solves this. It is a serializable snapshot of the
workspace intelligence produced by a running LSP server. lsp-max exports LSIF 0.6 NDJSON on
demand via `max/lsif`. The export includes the standard LSIF graph (documents, ranges, hovers,
references, monikers) plus lsp-max extensions: `ConformanceNode` vertices and `ConformanceEdge`
edges.

A CI/CD gate that receives the LSIF export does not need a running LSP server. It parses the
NDJSON stream, locates `ConformanceNode` vertices, checks the `refused` array, and exits 0 or 1.
An agent receiving a handoff from a previous agent does not need to reinitialize the LSP server
and re-analyze the workspace. It reads the LSIF export and knows which law axes are admitted,
refused, and unknown.

**Why ConformanceEdge multi-in is the key innovation.** Standard LSIF edges are point-to-point:
one source vertex, one target vertex. LSIF 0.6 introduces multi-in edges: one edge with one
source vertex and an array of target vertices (`inVs`). lsp-max's `ConformanceEdge` uses this to
link one `ConformanceNode` (carrying the law-state for a group of documents) to multiple document
vertices simultaneously.

This matters because law-state is often cross-document. A `Type` axis violation in a crate's
`lib.rs` may be caused by a `Cargo.toml` dependency declaration. The `ConformanceNode` for that
law axis must reference both documents. Without multi-in edges, this requires one ConformanceEdge
per document — N edges for N documents, with no single structure capturing "these N documents
together violate this law axis." With multi-in edges, one `ConformanceEdge` captures the
cross-document relationship. Consumers can query: "which documents are covered by this
ConformanceNode?" and receive a complete answer.

The `receipt_id` field in `ConformanceNode` links the law-state assertion to the cryptographic
receipt that proves it. This makes LSIF exports auditable: a CI gate can verify that the
ConformanceNode's claim is backed by a valid receipt chain, not just asserted.

---

## The three-protocol stack: A2A + MCP + LSP

The three protocols are not alternatives. They are complementary layers:

**A2A — who does what.** A2A handles agent-to-agent task delegation. Agent A receives a task too
large to complete alone. It decomposes the task and delegates sub-tasks to Agents B and C via A2A.
Each agent returns a structured result. A2A handles capability negotiation, handoff artifacts, and
task routing. It does not know or care whether the work being done is lawful.

**MCP — what tools exist.** MCP gives agents a uniform interface to tools and context servers.
An agent asks an MCP server "what files are in this directory?" or "what is the current git
status?" MCP handles tool discovery, parameter schemas, and response formatting. It does not
observe file mutations; it answers point queries.

**LSP — is the work lawful?** LSP observes every file mutation and continuously maintains
law-state. It does not handle task delegation (A2A's job) or tool discovery (MCP's job). It
handles the question "as the agent edits this workspace, is the law being followed?"

In a well-architected coding agent system, all three are present:
- A2A routes the task to the right agent
- MCP provides the agent with context and tools
- LSP enforces law-state throughout the agent's work

lsp-max's MCP bridge (`crates/lsp-max-mcp/`) is a pragmatic adapter for agents that speak only
MCP. It exposes `lsp_gate_check`, `lsp_route`, `lsp_health`, and `lsp_discover` as MCP tools.
But the bridge is point-in-time — it answers "what is the gate state right now?" It cannot push
a law signal the moment a file changes. For continuous ambient enforcement, the agent must speak
LSP directly.

---

## Why Unknown must never collapse: the three-valued logic

Standard boolean logic has two states: compliant and non-compliant. This is insufficient for
law-state in evolving codebases.

Consider a workspace where:
- The `Type` law axis is fully traced — every module that handles types has been analyzed, and all
  are clean. `Type` is `admitted`.
- The `Security` law axis has been partially traced — some modules have been analyzed, but others
  have not. The security scanner has not run against the new authentication module added yesterday.

In a two-state model, `Security` would be either `admitted` (optimistic, wrong) or `refused`
(pessimistic, also wrong). Neither is accurate. The accurate statement is "we do not know whether
Security is admitted for the authentication module — we have not looked."

`Unknown` captures this accurately. It means: "the evidence required to make a claim in either
direction has not been gathered." Unknown is not a soft Admitted. Unknown is not a soft Refused.
It is a distinct epistemic state.

The practical consequences of collapsing Unknown are severe:

**Collapsing Unknown to Admitted** causes false releases. An agent that treats "we haven't checked
Security" as "Security is clean" will release artifacts with untraced security properties. The
release gate passes because Unknown was silently promoted to Admitted.

**Collapsing Unknown to Refused** causes false blocks. An agent that treats "we haven't checked
Documentation" as "Documentation is violated" blocks legitimate releases. The development loop
degrades as agents spend time resolving false blocks.

The `admits_release()` predicate handles Unknown explicitly:
- In `strict_mode = false`: Unknown axes are tolerated. The agent may proceed but must record
  the Unknown axes in the handoff artifact. The receiving agent knows what was not checked.
- In `strict_mode = true`: Unknown axes block release. Nothing ships until all axes are traced.

The `strict_mode` flag lets the system calibrate to the risk tolerance of the domain. Development
sessions use `strict_mode = false`; production release gates use `strict_mode = true`.

---

## The OCEL process mining loop: trust event evidence, not code paths

"The code looks correct" is not sufficient evidence that the process was lawful. An agent can
produce output that appears correct through an illegal path: hardcoded values that look like
computed results, bypassed validation steps, test fixtures that fake conformance evidence.

The Van der Aalst process mining doctrine addresses this directly: **do not trust code paths,
state machines, or API responses. Trust only event evidence that can be mined into a conforming
object-centric process.**

lsp-max implements this doctrine through three layers:

**OCEL 2.0 accumulation.** Every significant state transition in the compositor emits an OCEL
2.0 event: `CompositorFlush`, `CompositorFlushAdmitted`, `CompositorFlushBlocked`, `AndonCodePresent`.
Every diagnostic emitted by a child server contributes an activity event: `ForbiddenRefDetected`,
`VictoryLanguageDetected`, etc. These events are accumulated by `FlushCoordinator` and drainable
via `take_ocel_events()`.

**Declare conformance checking.** The accumulated events are organized into traces (one trace per
case — typically per document or per session). The traces are checked against the normative
`DeclareModel::compositor()` — a 9-constraint LTL-style specification of the lawful flush
pipeline. A violation means the actual runtime process deviated from the declared process. The
code may look correct; the event log reveals the deviation.

**DFG discovery and fitness.** The Directly-Follows Graph is discovered from the traces:
`DFG = {(A, B) : B occurs immediately after A in at least one trace}`. The DFG is compared
against the normative arc set. Fitness measures what fraction of the normative arcs appear in
the DFG; precision measures what fraction of the DFG arcs are in the normative model.

Low fitness means expected transitions did not occur — the process was incomplete. Low precision
means unexpected transitions occurred — the process had rework loops, retries, or deviations not
in the normative model.

An agent cannot fake DFG fitness. The DFG is derived from the event log, which is derived from
runtime observations. An agent that skips the repair step produces a DFG missing the
`ForbiddenRefDetected → RepairApplied` arc — fitness drops, the Declare constraint
`Response(ForbiddenRefDetected, RepairApplied)` fires a violation, and the gate refuses release.

The `anti-llm://process-model` virtual document surfaces a live DFG as a Mermaid flowchart plus
a Declare conformance report. This gives agents (and developers reviewing agent sessions) a
continuous view of the actual runtime process, not the declared one.

---

## The ggen + RulePackServer pattern: domain law enforcement from RDF ontologies

lsp-max's `RulePackServer` trait is the entry point for domain-specific law enforcement. But
writing a new LSP server from scratch — even with the trait defaulting most of the boilerplate —
is substantial work. The `ggen` code generation system addresses this.

`ggen` reads an RDF ontology (`.ttl`), a SPARQL query (`.rq`), and a Tera template (`.tera`).
The SPARQL query extracts the relevant classes and properties from the ontology. The template
renders Rust source from the query results. The output is a first-class Rust source file — not
a generated file in a `generated/` directory, not a file with a `DO NOT EDIT` banner. It is
source that is inspected, reasoned about, and repaired like any other source.

For `RulePackServer`-based LSP servers, the pattern is:

1. Author an RDF ontology for the domain law (e.g., `ontology/lsif06.ttl` for LSIF 0.6 law)
2. Write a SPARQL query that extracts the diagnostic rules (codes, messages, law axes)
3. Write a Tera template that renders the `scan_uri_classified` implementation
4. `ggen sync` generates the Rust source
5. `ggen-lsp` validates the template/query binding live — GGEN-TPL-001 fires if a template
   variable does not appear in the SELECT output

The `ggen-lsp` server itself runs as a `DiagnosticsOnly` child server in the compositor. It
watches `.ttl`, `.rq`, `.tera`, and `ggen.toml` files. When a template binding breaks — the
SPARQL query is refactored and a variable is renamed — `ggen-lsp` immediately pushes `GGEN-TPL-001`.
The ANDON gate fires. The agent is blocked from calling `ggen sync` until the binding is repaired.
This prevents `ggen sync` from producing silently incorrect source from a broken binding.

The ontology-driven approach has a deeper benefit: the domain law is expressed in a formal,
queryable representation. The RDF ontology can be reasoned over with OWL/SPARQL. New constraints
can be added to the ontology and automatically propagated to the generated scanner. The ontology
is the authoritative definition of the domain law; the generated scanner is a derived artifact
of that definition.

`ontology/lsif06.ttl` (348 lines, OWL) is the reference example: it models all 24 LSIF 0.6
vertex types, 21 edge types, and the lsp-max extension types (`ConformanceNode`,
`ConformanceEdge`) as OWL classes with RDFS labels and comments. SPARQL queries over this
ontology generate the LSIF 0.6 validation logic in the lsp-max-lsif crate.

The `ggen` + `RulePackServer` pattern scales: each new domain (process mining, type authority,
receipt law, OCEL lifecycle) gets an ontology, a query, and a template. The compositor wires the
generated server. Domain law enforcement is systematic, not ad hoc — the ontology is the law;
the server is the witness; the OCEL event log is the proof.
