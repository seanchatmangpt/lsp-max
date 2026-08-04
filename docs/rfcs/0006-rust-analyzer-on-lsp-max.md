# RFC 0006: Rust Analyzer on the lsp-max Law-State Runtime

**Status:** Proposed

## Decision summary

Build `ra-max` as a new Rust language-analysis product whose semantic behavior is initially derived from the exact rust-analyzer source identified below, while its protocol lifecycle, project admission, effectful operations, evidence, replay, and standing are manufactured by `lsp-max`.

This is a rewrite of the **runtime architecture and authority model**, not an immediate rewrite of every rust-analyzer semantic algorithm.

The first lawful system therefore:

1. preserves rust-analyzer's parser, syntax, incremental database, HIR, and IDE feature boundaries behind an immutable semantic facade;
2. replaces the `crates/rust-analyzer` JSON-RPC/LSP shell with `lsp-max::LanguageServer`, `LspService`, routing, diagnostics, snapshots, conformance, and receipt primitives;
3. moves Cargo, rustc, rustfmt, flycheck, proc-macro, filesystem-write, and server-initiated edit operations behind one brokered and receipted actuation path;
4. records admitted project identity, semantic snapshot identity, constructed edits, actuation consequences, and replay material without putting cold semantic storage in the keystroke path; and
5. permits later replacement of rust-analyzer components only when an exact-subject differential harness proves the replacement against the pinned semantic baseline.

The target is not “rust-analyzer wrapped in receipt middleware.” The target is a Rust analysis factory in which selection, construction, and actuation have distinct authority and every external consequence has a bound receipt.

## Source identities

This RFC is grounded against immutable source commits:

| Subject | Repository | Commit |
|---|---|---|
| Target runtime | `seanchatmangpt/lsp-max` | `3c3e559347ce261a71435c98f34a0c171f51d592` |
| Semantic reference | `rust-lang/rust-analyzer` | `5f258f4534e3b4bdaa45a1299b53a66cf014d803` |

Moving either source identity requires a new admission record and a fresh differential result. A branch name, release label, or “latest” reference is not sufficient identity.

## Context

rust-analyzer and `lsp-max` solve different parts of the problem.

rust-analyzer is a mature incremental Rust compiler front end for IDE use. Its architecture deliberately separates:

- parser events from syntax-tree representation;
- per-file syntax from Cargo and filesystem concerns;
- Salsa ground inputs from derived semantic state;
- internal HIR computation from the public `hir` facade;
- semantic analysis from editor-facing `ide` POD values; and
- the semantic engine from the sole crate that knows LSP and JSON serialization.

`lsp-max` is a law-state LSP runtime. It provides protocol transport, lifecycle, composition, snapshots, conformance vectors, diagnostics, receipt chains, LSIF, ontology storage, and explicit gates for machine-agent workflows.

A naive combination fails in two opposite directions:

1. **Wrap rust-analyzer's current server.** This leaves Cargo invocations, proc-macro execution, edits, formatting, and state transitions under rust-analyzer's ambient authority. Receipts become observational decoration after the actual consequence.
2. **Replace rust-analyzer's semantic engine with Tree-sitter rules.** This discards the hard-won Rust-specific parser, macro expansion, name resolution, type inference, completion, assists, and incremental invalidation topology. Syntax similarity is not semantic equivalence.

The lawful design preserves the semantic graph first, fences every effect, and then replaces components only where equivalence has been observed against the same subject and boundary.

## Chesterton fences

The following rust-analyzer boundaries are preserved until a replacement passes the differential verifier.

### Parser independence

The parser remains independent of a particular syntax-tree representation and continues to produce recoverable syntax plus errors. Parsing must remain available on incomplete and malformed source.

Tree-sitter may be used as a separate structural oracle, fast prefilter, or foreign-language adapter. It does not replace the Rust parser in the semantic authority path merely because both produce trees.

### Syntax as a value

A syntax tree remains a per-file value determined by file contents. Semantic state is not attached to syntax nodes. Refactors and assists may construct new trees without mutating globally interned semantic objects.

`TreeSitterTree` remains outside Salsa tracked outputs. A rust-analyzer/Rowan syntax value may cross the semantic facade only through bounded immutable snapshots; parser implementation objects do not become runtime authority tokens.

### Ground state versus derived state

The semantic database accepts admitted source text, crate graph, cfg flags, environment, toolchain facts, and proc-macro facts as ground input. Name resolution, macro expansion, type inference, references, diagnostics, and IDE features are derived.

Cargo concepts are lowered before entering the semantic database. `base-db` does not gain filesystem or Cargo authority.

### Incremental invalidation

Editing a function body must not invalidate unrelated global facts. The rewrite cannot claim parity merely because results are correct after a full recomputation; it must preserve bounded incremental behavior.

### LSP isolation

Only the `ra-max-server` boundary knows LSP and JSON serialization. Semantic crates return editor-domain POD values and never manufacture protocol messages or receipts directly.

### Broken-build availability

A broken Cargo build, failed flycheck, unavailable proc macro, or refused external process must not make syntax and already-admitted semantic features unavailable. Failures remain typed evidence and never collapse into false semantic facts.

## Non-equivalences that must remain explicit

The systems use several similar words for different objects. The rewrite must not merge them by name.

| rust-analyzer object | lsp-max object | Required distinction |
|---|---|---|
| compiler or IDE diagnostic | ANDON refusal | A diagnostic is observed analysis evidence. It becomes ANDON only when a governed transition is refused. |
| `Analysis` snapshot | receipt | A snapshot is immutable semantic state. A receipt binds identity, authority, consequence, and replay for a transition involving that snapshot. |
| `SourceChange` or `WorkspaceEdit` | actuation | An edit value is a constructed candidate. Actuation occurs only when an authorized party applies it. |
| Cargo metadata | admitted project model | Command output is raw observation until command identity, environment, exit status, parser version, and project bounds are admitted. |
| successful query | standing | A query result can be correct without proving that any external state changed. |
| LSP response sent | effect completed | Sending an edit or command to a client does not prove the client applied it. |
| test output | receipt | Test output is evidence input. A receipt binds it to the exact source, validator, toolchain, configuration, and consequence. |

## Foundational calculus

### Objects

The system manipulates the following first-class objects:

- `RawWorkspaceObservation`: files, manifests, environment, toolchain probes, Cargo metadata bytes, proc-macro inventory, and client configuration as observed;
- `AdmittedWorkspace`: canonical project identity and bounded semantic inputs;
- `SemanticRevision`: immutable semantic database snapshot;
- `QueryIntent`: a read-only semantic request;
- `ConstructionIntent`: a request to manufacture completion text, assists, source changes, command plans, or repair plans;
- `ActuationIntent`: an authorized request to cause an external consequence;
- `DiagnosticFact`: syntax, type, linter, project, or runtime evidence;
- `Refusal`: a typed rejection at an admission or actuation boundary;
- `Receipt`: the binding of subject, authority, consequence, verifier, and replay material; and
- `ReplayCapsule`: sufficient source and event material to reproduce a bounded transition.

### Morphisms

```text
observe
  RawWorkspaceObservation

admit
  RawWorkspaceObservation -> AdmittedWorkspace | Refusal

revise
  AdmittedWorkspace x SourceDelta -> SemanticRevision | Refusal

select
  SemanticRevision x QueryIntent -> QueryResult

construct
  SemanticRevision x ConstructionIntent -> ConstructedArtifact

actuate
  AuthorizedPrincipal x ConstructedArtifact -> Consequence | Refusal

receipt
  Subject x Authority x Consequence x Verifier -> Receipt

replay
  ReplayCapsule x ValidatorIdentity -> ReplayResult
```

Only `actuate` is permitted to cause filesystem, process, network, client-state, or release-state consequences. `select`, `construct`, model output, hooks, proofs, diagnostics, and semantic derivations have no ambient actuation authority.

### Closure

A request is closed only when every required boundary has one of these outcomes:

- observed and admitted;
- executed and verified;
- refused with a typed reason;
- unsupported with a bounded capability explanation; or
- blocked by a named missing dependency or authority.

Unknown is preserved. Unsupported is not refusal. A constructed candidate is not executed. A sent command is not an observed consequence.

## Target architecture

```text
Editor / Agent / Replay Driver
            |
            v
+------------------------------------------------------------+
| ra-max-server                                              |
| lsp-max LanguageServer + LspService + lifecycle + routing  |
+------------------------------+-----------------------------+
                               |
                 parse / route / classify
                               |
          +--------------------+--------------------+
          |                                         |
          v                                         v
+---------------------------+          +---------------------------+
| SELECT                    |          | CONSTRUCT                 |
| hover, goto, refs, symbols|          | completion, assist, edit  |
| semantic tokens, inlay    |          | format plan, command plan |
+-------------+-------------+          +-------------+-------------+
              |                                      |
              +------------------+-------------------+
                                 v
                    +---------------------------+
                    | ra-max-semantic           |
                    | AnalysisHost / Analysis   |
                    | parser -> syntax -> HIR   |
                    | Salsa incremental queries |
                    +-------------+-------------+
                                  |
                        immutable POD results
                                  |
                                  v
                    +---------------------------+
                    | ra-max-admission          |
                    | VFS, crate graph, cfg,    |
                    | toolchain and config O*   |
                    +-------------+-------------+
                                  |
                    external consequence needed?
                                  |
                          no -----+----- yes
                                        |
                                        v
                    +---------------------------+
                    | ra-max-brce               |
                    | sole DO path              |
                    | cargo/rustc/rustfmt       |
                    | proc macro/file/edit/release|
                    +-------------+-------------+
                                  |
                                  v
                    +---------------------------+
                    | consequence + receipt     |
                    | OCEL + LSIF + cold RDF    |
                    | replay capsule            |
                    +---------------------------+
```

The semantic engine is downstream of admitted inputs and upstream of constructed artifacts. It does not own external process or file authority.

## Workspace decomposition

The rewrite is introduced as bounded crates rather than one new monolith.

### `crates/ra-max-server`

Responsibilities:

- implement `lsp_max::LanguageServer`;
- own LSP/JSON conversion;
- classify methods as SELECT, CONSTRUCT, or DO;
- bind each request to a semantic revision and cancellation token;
- route notifications into admission and revision updates;
- publish diagnostic facts separately from ANDON refusals; and
- expose `max/*` explanation, snapshot, receipt, replay, and gate methods.

It replaces rust-analyzer's current LSP shell and main loop. It does not implement parsing, HIR, type inference, or Cargo loading.

### `crates/ra-max-semantic`

Responsibilities:

- expose a small immutable facade over the pinned rust-analyzer semantic engine;
- own `AnalysisHost` and create `Analysis` snapshots;
- convert admitted project inputs into semantic database inputs;
- return POD query results independent of LSP and receipts;
- expose semantic revision identity and cancellation; and
- prevent rust-analyzer internal types from leaking across the API boundary.

Initial implementation may import an immutable mirror of the pinned rust-analyzer crates. That mirror is generated source: it is never the normal editing surface. All local behavior changes live in adapter or overlay crates.

### `crates/ra-max-admission`

Responsibilities:

- observe VFS, manifests, Cargo metadata, cfgs, sysroot, target data, environment, and client configuration;
- canonicalize observations into `AdmittedWorkspace`;
- hash exact bytes and identities;
- retain typed failures without converting them into semantic truth; and
- transact accepted source deltas into the semantic host.

An admission record includes at least:

```toml
schema = "ra-max.admission.v1"
workspace_root_hash = "blake3:..."
source_manifest_hash = "blake3:..."
crate_graph_hash = "blake3:..."
cfg_hash = "blake3:..."
toolchain_hash = "blake3:..."
proc_macro_inventory_hash = "blake3:..."
client_config_hash = "blake3:..."
semantic_reference = "rust-lang/rust-analyzer@5f258f4534e3b4bdaa45a1299b53a66cf014d803"
```

### `crates/ra-max-brce`

Responsibilities:

- be the only constructor of effect capabilities;
- authorize and execute Cargo, rustc, rustfmt, flycheck, proc-macro, filesystem-write, server-initiated edit, and release operations;
- capture command, arguments, cwd identity, environment policy, stdin hash, stdout/stderr hashes, exit status, timeout, resource bounds, and changed-object hashes;
- emit a consequence receipt or typed refusal; and
- manufacture no semantic facts merely because a command succeeded.

The broker accepts structured intents, never shell prose.

### `crates/ra-max-evidence`

Responsibilities:

- define semantic revision, construction, actuation, diagnostic-flush, and release receipts;
- append OCEL events;
- export LSIF for durable structural identity;
- project selected cold semantic facts to Oxigraph outside the hot path; and
- construct deterministic replay capsules.

### `crates/ra-max-differential`

Responsibilities:

- execute identical fixtures against the pinned rust-analyzer reference and `ra-max`;
- canonicalize nondeterministic protocol fields;
- compare semantic and protocol outputs;
- measure invalidation and latency; and
- emit a machine-readable verifier report.

The differential harness is the admission authority for replacing semantic components. Unit tests alone cannot crown equivalence.

## Semantic engine import strategy

The rewrite uses a three-stage source strategy.

### Stage 1: Immutable upstream mirror

Import the dependency-closed rust-analyzer semantic subset at the pinned commit. The initial subset includes the parser, syntax, base database, HIR layers, IDE facade and feature crates, project model, VFS, toolchain, macro support, and required utility crates.

The mirror is identified by a manifest and content digest. It is not hand-edited. Local patches are represented as explicit patch objects or adapter code so upstream correspondence remains inspectable.

### Stage 2: Boundary extraction

Stabilize `ra-max-semantic` around POD inputs and outputs. rust-analyzer internal crate topology remains hidden behind the facade. Salsa versions, Rowan types, HIR IDs, and proc-macro protocol objects do not cross into `lsp-max` runtime crates.

This avoids coupling `lsp-max`'s current Salsa dependency to rust-analyzer's Salsa version. Two versions may coexist temporarily because no tracked type crosses the facade. Unifying versions is a later optimization, not an admission prerequisite.

### Stage 3: Component manufacture

Replace one semantic component at a time only after:

1. the replacement has a bounded interface;
2. the exact reference component remains available to the differential runner;
3. fixtures cover positive, negative, incomplete-code, macro, cfg, and cancellation behavior;
4. output equivalence and performance budgets pass; and
5. the replacement receives a verifier receipt.

A failed replacement edge is topology, not graph failure. The remaining pinned components retain standing.

## Request authority classification

### SELECT

SELECT reads an immutable semantic revision and cannot change external state.

Examples:

- hover;
- go to definition, declaration, implementation, and type definition;
- references;
- document and workspace symbols;
- semantic tokens;
- folding ranges;
- inlay hints;
- signature help;
- call hierarchy; and
- read-only diagnostic pulls.

SELECT results may carry a semantic revision identity and evidence digest. They do not require an actuation receipt.

### CONSTRUCT

CONSTRUCT manufactures a candidate artifact but does not apply it.

Examples:

- completion items and completion resolve payloads;
- assists and code actions;
- rename edits;
- source changes;
- format edits;
- import insertion;
- repair plans; and
- commands represented as structured intents.

A construction receipt binds the semantic revision, request parameters, generated artifact hash, and constructor identity. It proves what was constructed, not that a client applied it.

### DO

DO causes or requests an external consequence under `ra-max` authority.

Examples:

- invoking Cargo, rustc, clippy, rustfmt, flycheck, or a proc-macro server;
- writing files;
- applying a server-initiated `workspace/applyEdit`;
- mutating durable indexes;
- installing or updating toolchains;
- publishing artifacts; and
- release actuation.

Every DO path goes through `ra-max-brce`. There is no direct process spawn, filesystem write, network request, or server-initiated client mutation elsewhere in the graph.

When the client applies an edit on its own authority, `ra-max` records only the constructed edit and later-observed document delta. It must not claim that the client applied the edit unless that consequence is observed and correlated.

## End-to-end flows

### Document change

```text
LSP didChange
  -> validate document identity and version
  -> admit ordered source delta
  -> update VFS ground input
  -> transact delta into AnalysisHost
  -> create SemanticRevision
  -> schedule bounded diagnostics
  -> stage DiagnosticFacts
  -> atomic diagnostic flush
  -> emit flush receipt
```

No Cargo command, proc macro, Oxigraph write, LSIF load, or filesystem write occurs synchronously in the keystroke path.

### Hover

```text
Hover request
  -> bind request to SemanticRevision
  -> SELECT semantic query
  -> convert POD result to LSP
  -> return response with revision correlation
```

The result has semantic provenance but no actuation standing because no external state changed.

### Rename

```text
Rename request
  -> SELECT symbol and reference graph
  -> CONSTRUCT SourceChange
  -> validate edit preconditions against document versions
  -> emit construction receipt
  -> return WorkspaceEdit
  -> later didChange observations confirm or contradict application
```

The server does not claim the rename happened from the returned `WorkspaceEdit` alone.

### Flycheck

```text
Flycheck intent
  -> admit command policy and workspace revision
  -> BRCE authorizes process capability
  -> execute bounded Cargo/rustc command
  -> capture outputs and exit status
  -> parse diagnostics as observations
  -> correlate diagnostics to admitted source revision
  -> publish diagnostic facts
  -> emit actuation and diagnostic-flush receipts
```

A failed process is `ToolFailed`, `TimedOut`, `Refused`, or `Unsupported`; it is never converted into a successful semantic result.

### Proc macro expansion

```text
Semantic query requires proc macro
  -> locate admitted proc-macro artifact
  -> BRCE authorizes isolated process/session
  -> execute bounded token-tree transform
  -> hash input/output and runtime identity
  -> admit expansion result or preserve typed failure
  -> feed admitted token tree to semantic revision
```

Proc-macro output has no filesystem or network authority. The process capability is narrower than the semantic meaning assigned to its returned token tree.

## Diagnostics and ANDON

The rewrite uses two channels.

### Diagnostic facts

Diagnostic facts report observed properties of source, project state, or tool execution:

- parser errors;
- unresolved names;
- type mismatches;
- unused items;
- lints;
- Cargo metadata failures;
- proc-macro failures; and
- flycheck output.

They may be errors without being refusals.

### ANDON refusals

ANDON is emitted only when a governed transition is blocked, including:

- stale document version used for an edit;
- unadmitted workspace identity;
- invalid or missing authority;
- attempted process spawn outside BRCE;
- stale LSIF loaded without a receipt;
- semantic-memory write without a bound revision receipt;
- cold ontology access attempted in the hot path;
- unknown conformance state silently collapsed; or
- release requested without complete verifier standing.

A diagnostic may lead to a refusal, but `DiagnosticFact -> ANDON` is not automatic.

## Storage topology

| Store | Role | Hot-path rule |
|---|---|---|
| VFS/source store | admitted text and file identity | allowed |
| Salsa | incremental derived computation | allowed; no external authority |
| diagnostic staging | atomic per-revision diagnostic batches | allowed; implementation hidden behind interface |
| LSIF | durable structural graph | load only with identity and freshness receipt |
| Oxigraph | cold semantic meaning and ontology projection | forbidden in `didChange` and latency-critical SELECT paths |
| OCEL | transition history | append asynchronously from bounded events |
| receipt store | standing and replay identity | synchronous only at required boundaries; durable persistence may be queued |

Salsa remembers computation. It does not become the durable audit log, filesystem, process supervisor, or ontology database.

## Receipt families

### Admission receipt

Binds raw observation hashes to `AdmittedWorkspace` and records every exclusion, refusal, unsupported capability, and bounded assumption.

### Semantic revision receipt

Binds source revision, crate graph, cfgs, toolchain, proc-macro inventory, semantic engine identity, and resulting semantic revision.

This receipt proves the revision's inputs. It does not prove every possible query result.

### Construction receipt

Binds semantic revision, request, constructor, preconditions, and artifact hash for a completion, assist, rename, edit, or command plan.

### Actuation receipt

Binds principal, authority, structured intent, command or mutation identity, observed consequence, changed objects, verifier, and replay material.

### Diagnostic flush receipt

Binds source revision, all contributing analyzers, diagnostic batch hash, publication sequence, and client channel identity.

### Differential verifier receipt

Binds reference source, candidate source, fixture corpus, canonicalizer, toolchain, configuration, output comparison, performance observations, and falsifiers.

### Release receipt

Binds exact source tree, generated-source manifest, validation pack, toolchain capsule, verifier DAG, artifact hashes, and release consequence.

## Replay model

Replay is event-sourced at the protocol and authority boundaries, not by serializing an opaque live process.

A replay capsule contains:

- admitted workspace manifest;
- initial file bytes or content-addressed references;
- ordered source deltas;
- configuration changes;
- project-model observations;
- semantic reference and toolchain identities;
- LSP requests and notifications with normalized timestamps and IDs;
- external command intents and captured results or deterministic fixtures;
- proc-macro input/output fixtures where lawful; and
- expected receipts and verifier report.

Replay modes:

1. **Pure semantic replay:** no external processes; uses admitted project and proc-macro fixtures.
2. **Broker replay:** re-executes bounded external operations and compares consequences.
3. **Differential replay:** executes reference rust-analyzer and `ra-max` against the same event stream.
4. **Release replay:** reconstructs the exact validation DAG and artifact identities.

## Migration plan

### Phase 0: Reference capsule

- mirror the exact rust-analyzer semantic source closure;
- record license and source manifest;
- build a protocol trace corpus from representative Rust workspaces;
- classify nondeterministic fields; and
- establish reference latency, memory, cancellation, and invalidation observations.

Exit criterion: the reference itself replays deterministically under the canonicalizer, or every residual nondeterminism is bounded and named.

### Phase 1: Read-only vertical slice

Implement:

- initialize/shutdown lifecycle on `lsp-max`;
- admitted single-workspace project model;
- didOpen/didChange/didClose revision flow;
- syntax diagnostics;
- hover;
- go to definition; and
- semantic snapshot and diagnostic-flush receipts.

No server-side filesystem writes or external process execution are admitted.

Exit criterion: differential equality for the vertical-slice corpus and zero DO edges outside BRCE.

### Phase 2: Semantic parity surface

Add references, completion, semantic tokens, inlay hints, symbols, call hierarchy, diagnostics, and cancellation. Preserve rust-analyzer's partial availability under broken builds.

Exit criterion: all admitted SELECT methods pass differential replay and latency budgets.

### Phase 3: Construction surface

Add assists, code actions, rename, source changes, formatting plans, and command plans with document-version preconditions and construction receipts.

Exit criterion: constructed artifacts match canonical rust-analyzer outputs or have an explicit admitted divergence.

### Phase 4: Brokered effects

Move Cargo metadata, flycheck, rustfmt, proc macros, filesystem mutation, server-initiated edits, and release operations behind BRCE.

Exit criterion: static and runtime guards demonstrate zero unreceipted actuation.

### Phase 5: Durable graph and replay

Add LSIF export/import, cold RDF projection, OCEL history, replay capsules, and release receipt DAGs.

Exit criterion: exact-subject replay reconstructs the admitted semantic and actuation outcomes.

### Phase 6: Semantic component replacement

Replace pinned rust-analyzer components only where the replacement improves a named objective and passes component plus system differential verification.

Exit criterion: each replaced component has independent verifier standing; unreplaced components remain pinned.

## First implementation slice

The first code PR after this RFC should remain small and executable. It should add only:

```text
crates/ra-max-server/
crates/ra-max-semantic/
crates/ra-max-admission/
crates/ra-max-differential/
```

Required behavior:

1. start an `lsp-max` server over stdio;
2. admit one fixture workspace without invoking Cargo;
3. accept `initialize`, `initialized`, `didOpen`, `didChange`, `hover`, `definition`, `shutdown`, and `exit`;
4. answer hover and definition through a semantic facade;
5. publish syntax diagnostics as diagnostic facts;
6. emit semantic-revision and diagnostic-flush receipts; and
7. replay the same trace to the same canonical outputs.

The first slice must not add formatting, code actions, filesystem writes, Cargo, proc macros, LSIF loading, Oxigraph access, or release actuation.

## Acceptance contract

A milestone is `ALIVE` only when execution has been observed against the exact admitted subject. The following gates are cumulative.

### Identity

- target `lsp-max` base commit recorded;
- rust-analyzer reference commit recorded;
- imported semantic source closure content-addressed;
- generated and mirrored files classified; and
- toolchain and configuration identities recorded.

### Architecture

- no LSP types cross into semantic crates;
- no semantic internal tracked types cross into runtime crates;
- no process spawn outside BRCE;
- no filesystem write outside BRCE;
- no Oxigraph query in document-change or latency-critical SELECT paths;
- no stale LSIF load without a receipt; and
- diagnostic facts and ANDON refusals remain distinct.

### Behavioral parity

- identical admitted fixture and event stream used for reference and candidate;
- canonical outputs compared for every admitted method;
- incomplete code, cfg variants, macro expansion, cancellation, broken build, and stale-version cases included;
- divergences are either repaired or explicitly admitted with rationale; and
- no “passes unit tests” substitution for protocol differential proof.

### Incrementality and performance

- recomputation topology observed for bounded edits;
- unrelated global facts remain reusable after function-body edits;
- cancellation prevents stale result publication;
- hot-path receipt work is bounded; and
- candidate latency and memory are compared to the pinned reference on the same machine and corpus.

The initial performance admission budget is relative rather than universal:

- no admitted SELECT operation may regress p95 latency by more than 10% against the pinned reference without a named waiver; and
- didChange-to-diagnostic-flush p95 may not regress by more than 15% without a named waiver.

A waiver is `PARTIAL_ALIVE`, never full parity.

### Authority and receipts

- every DO operation has an authorization decision;
- every admitted DO operation has an observed consequence or typed failure;
- receipt identity includes source, validator, toolchain, configuration, authority, and consequence;
- constructed edits are not reported as applied edits;
- client-applied consequences are correlated only after observation; and
- replay verifies receipt bindings.

## Falsifiers

Any of the following falsifies the architecture claim:

1. a process, file, network, client-state, durable-index, or release mutation occurs outside BRCE;
2. a rust-analyzer diagnostic is automatically represented as a refusal;
3. a returned `WorkspaceEdit` is reported as an applied change without correlated observation;
4. Cargo metadata or proc-macro output enters semantic ground state without admission identity;
5. Tree-sitter output replaces the Rust semantic parser without differential equivalence proof;
6. a full recomputation is presented as incremental parity;
7. a semantic component is removed before the reference edge exists in the differential harness;
8. cold ontology storage appears in the keystroke path;
9. an unknown capability silently becomes supported or refused;
10. a receipt is synthesized from claims rather than bound to observed execution; or
11. the compared rust-analyzer or `lsp-max` source moves from the recorded commit without re-admission.

## Exclusions

This RFC does not authorize:

- a big-bang handwritten reimplementation of rust-analyzer HIR or type inference;
- copying rust-analyzer files into ordinary editable source without an immutable source manifest;
- changing rust-analyzer behavior merely to simplify `lsp-max` integration;
- running Cargo or proc macros directly from semantic query code;
- treating hooks, models, N3 rules, diagnostics, proofs, or code actions as actuation authority;
- synchronous RDF or process-mining work on every keystroke;
- claiming client-side effects that the server cannot observe;
- publishing crates or releases; or
- merging the implementation without exact-head local and CI receipts.

## Consequences

### Positive

- rust-analyzer's mature semantic capability is preserved while its ambient effects become explicit and receipted;
- the migration has an executable vertical slice rather than a speculative fork;
- semantic replacement is incremental, reversible, and differential;
- source identity and upstream correspondence remain inspectable;
- agents receive deterministic SELECT, CONSTRUCT, DO, refusal, and replay boundaries; and
- diagnostics, edits, commands, and receipts stop being conflated.

### Negative

- the initial system carries both rust-analyzer and `lsp-max` abstractions;
- the semantic facade requires disciplined POD conversion;
- two Salsa versions may coexist until a later admitted convergence;
- the differential corpus and canonicalizer become permanent maintenance obligations;
- proc-macro and Cargo sandboxing add latency and operational complexity; and
- immutable upstream mirroring increases repository or artifact size.

### Neutral

- LSP wire compatibility can remain standard for existing editors;
- `max/*` methods are additive and may be ignored by ordinary clients;
- rust-analyzer remains the behavioral oracle during the migration, not the permanent architectural authority; and
- later semantic innovations remain possible because the facade is reference-independent.

## Alternatives considered

### Run rust-analyzer as a child LSP server under the compositor

Useful as a baseline and fallback, but insufficient as the target. The child process retains ambient authority and opaque internal state. The compositor can receipt messages but cannot prove the internal project admission, query revision, or external effects.

### Fork rust-analyzer and replace its transport only

Closer, but still leaves effect authority distributed across project model, flycheck, toolchain, proc-macro, formatting, and server code. The result would be a protocol fork, not a law-state rewrite.

### Replace the semantic engine with Tree-sitter plus rule packs

Rejected as a starting point. Tree-sitter supplies robust structural parsing but is not equivalent to Rust macro expansion, name resolution, type inference, IDE completion, or incremental semantic invalidation.

### Rebuild every semantic crate from first principles

Potentially valid as a long-term research program, but not the fastest defensible local `ALIVE`. It destroys the reference topology before a replacement has standing.

### Keep rust-analyzer unchanged and add audit logging

Rejected. Audit logging after effects does not establish admission authority, zero unreceipted actuation, or exact replay.

## Operational standing

Approval of this RFC authorizes architecture work and the bounded Phase 1 vertical slice only. It does not claim semantic parity, broker completeness, or release readiness.

Current standing at RFC creation:

- target sources: **observed**;
- architecture boundaries: **admitted for proposal**;
- implementation: **UNKNOWN**;
- differential execution: **UNKNOWN**;
- zero-unreceipted-actuation proof: **UNKNOWN**;
- release standing: **UNSUPPORTED by this RFC**.

## References

- [`AGENTS.md`](../../AGENTS.md)
- [`docs/book/01-architecture.md`](../book/01-architecture.md)
- [RFC 0002: Law Enforcement via Receipt Chains](0002-law-enforcement-via-receipt-chains.md)
- [RFC 0003: Conformance Vector Three-Valued Logic](0003-conformance-vector-three-valued-logic.md)
- [RFC 0004: Composition Over tower-lsp Fork](0004-composition-over-tower-lsp-fork.md)
- rust-analyzer architecture at `rust-lang/rust-analyzer@5f258f4534e3b4bdaa45a1299b53a66cf014d803`
