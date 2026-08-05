# RFC 0006: Rust Analyzer on the lsp-max Law-State Runtime

**Status:** Proposed — executable 80/20 vertical slice implemented

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
- IDE operations from the JSON-RPC transport; and
- a mutable world state from immutable analysis snapshots.

`lsp-max` supplies a different substrate:

- complete generated LSP vocabulary and routing;
- law-state lifecycle admission;
- conformance vectors and ANDON semantics;
- process evidence, receipts, and replay;
- LSIF and ontology projection; and
- explicit actuation boundaries.

A lawful rewrite must preserve the semantic properties that make rust-analyzer useful while replacing ambient runtime authority with admitted and receipted authority.

## Preservation fences

### Parser-event fence

rust-analyzer's parser emits events that are later converted to a lossless syntax tree. Grammar recovery, token attachment, and error placement depend on this split. Replacing it with a direct AST builder before equivalence proof would destroy syntax fidelity and error recovery.

### Syntax-value fence

Syntax nodes are immutable value-like handles. Semantic code may retain syntax identities without granting the syntax layer filesystem, Cargo, process, or network authority.

### Ground-state fence

Source text, file identities, crate graph, proc-macro results, and toolchain configuration are ground inputs. Derived queries must not silently mutate them.

### Incremental-invalidation fence

Salsa query invalidation is part of the semantic contract. A replacement is not equivalent merely because it returns the same answer once; it must preserve bounded recomputation behavior under edits.

### LSP-isolation fence

The semantic engine must not depend on `lsp_types` or JSON-RPC transport types. Protocol adaptation remains outside the semantic facade.

### Broken-build fence

Rust projects are routinely incomplete or temporarily invalid. The analysis runtime must preserve partial semantic standing instead of collapsing the whole workspace into failure.

## Explicit non-equivalences

The following terms are not interchangeable:

| Existing object | Not equivalent to | Required distinction |
|---|---|---|
| rust-analyzer diagnostic | `lsp-max` ANDON | A diagnostic describes source state; ANDON describes manufacturing/runtime standing. |
| immutable analysis snapshot | receipt | A snapshot supports queries; a receipt binds identity, authority, consequence, and replay. |
| `WorkspaceEdit` construction | edit actuation | Constructing an edit has no authority to mutate a client or filesystem. |
| Cargo/rustc output | admitted project state | Tool output becomes ground state only after admission and identity binding. |
| LSP notification | durable event | Transport delivery alone does not prove consequence or replay standing. |
| successful query | semantic equivalence | One result does not prove incremental, cancellation, malformed-input, or performance equivalence. |

## Target architecture

```text
Editor / IDE client
        |
        v
+---------------------------+
| ra-max-server             |
| lsp-max routing/lifecycle |
+-------------+-------------+
              |
              v
+---------------------------+
| ra-max-admission          |
| project/toolchain O*      |
+-------------+-------------+
              |
              v
+---------------------------+
| ra-max-semantic facade    |
| pinned RA engine first    |
+-------------+-------------+
              |
       SELECT | CONSTRUCT
              v
+---------------------------+
| ra-max-brce               |
| exclusive DO path         |
+-------------+-------------+
              |
              v
+---------------------------+
| ra-max-evidence           |
| receipts/replay/standing  |
+---------------------------+

       +---------------------+
       | ra-max-differential |
       | baseline/candidate  |
       +---------------------+
```

### Crate responsibilities

#### `ra-max-server`

- owns `lsp-max::LanguageServer` implementation;
- maps LSP requests to semantic facade operations;
- publishes diagnostics and progress;
- constructs client edits but does not actuate them directly;
- exposes replay and standing inspection methods.

#### `ra-max-admission`

- admits workspace roots, manifests, toolchain identity, configuration, and environment bounds;
- normalizes path and URI identity;
- records file-set and configuration hashes;
- rejects unbounded or unsupported project states with typed refusals.

#### `ra-max-semantic`

- initially presents a facade over a pinned rust-analyzer semantic engine;
- owns immutable revisions and query inputs;
- exposes diagnostics, hover, navigation, completion, assists, symbols, semantic tokens, and project status as constructed values;
- has no process, network, or direct mutation authority.

#### `ra-max-brce`

- is the exclusive path for filesystem writes, subprocess launch, proc-macro execution, client-applied workspace edits, and release operations;
- commits authorization before actuation;
- binds consequence after observed execution;
- emits typed refusal or failure receipts.

#### `ra-max-evidence`

- stores admission, semantic-revision, construction, actuation, diagnostic-flush, differential, and release receipts;
- supports deterministic replay and standing queries;
- keeps cold evidence outside the keystroke hot path.

#### `ra-max-differential`

- executes identical admitted inputs against a pinned baseline and candidate component;
- compares normalized results, diagnostics, edit constructions, and invalidation behavior;
- records an equivalence or divergence receipt;
- blocks component replacement on unexplained divergence.

## Authority calculus

### SELECT

SELECT operations may choose existing admitted information but cannot manufacture external consequences.

Examples:

- choose the crate owning a file;
- select a definition candidate;
- select diagnostics for a URI;
- choose a completion ranking policy.

### CONSTRUCT

CONSTRUCT operations manufacture reversible values without ambient execution authority.

Examples:

- semantic snapshots;
- hover content;
- completion lists;
- code actions;
- `WorkspaceEdit` values;
- Cargo/rustc command intents;
- release plans.

### DO

DO changes machine, client, process, network, or durable external state. DO is legal only through BRCE.

Examples:

- write a source file;
- launch Cargo, rustc, rustfmt, flycheck, or a proc macro;
- request client-side application of a workspace edit;
- publish a release;
- upload an artifact.

## Hot and cold state

### Hot semantic path

The edit-to-diagnostic and edit-to-completion path retains only data needed for interactive latency:

- document text and incremental syntax;
- Salsa ground inputs and memoized queries;
- current semantic revision identity;
- bounded in-memory request context;
- lightweight receipt-chain heads.

### Cold evidence path

The following remain off the keystroke path:

- complete receipt bodies;
- replay capsules;
- normalized differential outputs;
- LSIF and RDF projections;
- audit exports;
- historical semantic revisions.

The hot path may enqueue evidence material but may not wait for ontology or archive projection unless the requested operation explicitly requires it.

## Project admission

An admitted project record contains at least:

```rust
pub struct AdmittedRustProject {
    pub project_hash: String,
    pub root_uri: String,
    pub manifest_hashes: Vec<String>,
    pub source_set_hash: String,
    pub toolchain_hash: String,
    pub rustc_identity: String,
    pub cargo_identity: String,
    pub target_triples: Vec<String>,
    pub environment_hash: String,
    pub semantic_baseline_sha: String,
}
```

Admission must bound:

- workspace roots and path traversal;
- file count and aggregate source bytes;
- target count;
- Cargo metadata size;
- proc-macro execution policy;
- build-script execution policy;
- environment variables visible to tools;
- external command duration and output size.

## Semantic revision

Every query result is bound to a semantic revision:

```text
revision_hash = BLAKE3(
    admitted_project_hash ||
    source_input_hashes ||
    configuration_hash ||
    proc_macro_result_hashes ||
    semantic_engine_identity
)
```

A request may be answered only from a revision that matches its admitted subject. A result from a stale revision is not silently upgraded to current standing.

## Brokered execution flows

### Cargo and rustc

```text
LSP request or file event
  -> construct ToolIntent
  -> admit executable/toolchain/environment/bounds
  -> authorization receipt
  -> BRCE launches process
  -> observe exit/stdout/stderr/artifacts
  -> consequence receipt
  -> admit selected output as new ground state
  -> semantic revision
```

### Proc macros

Proc macros receive no ambient process authority from the semantic engine.

```text
macro query
  -> construct ProcMacroIntent
  -> apply project policy and resource bounds
  -> authorization receipt
  -> isolated execution
  -> hash output and diagnostics
  -> consequence receipt
  -> admit result into semantic revision
```

### Workspace edits

```text
semantic query
  -> construct WorkspaceEdit
  -> construction receipt
  -> return edit to client
  -> optional applyEdit intent
  -> authorization receipt
  -> client request
  -> observed response
  -> consequence receipt
  -> wait for document observation
  -> new semantic revision
```

A positive `workspace/applyEdit` response does not itself prove that a later document observation matches the proposed edit.

## Diagnostics and ANDON

Diagnostics and manufacturing standing are separate channels.

### Source diagnostics

- syntax errors;
- unresolved names;
- type errors;
- borrow-checking errors;
- lints;
- project-model diagnostics.

### ANDON conditions

- stale project admission;
- toolchain identity drift;
- unreceipted attempted actuation;
- proc-macro timeout;
- receipt-chain break;
- semantic baseline divergence;
- replay mismatch;
- unsupported target or build-script policy.

A project may have source diagnostics while the runtime is ALIVE. The runtime may also be BLOCKED while a previously computed source diagnostic remains valid for an older semantic revision.

## Differential replacement protocol

Semantic replacement occurs component by component.

For each candidate component:

1. pin baseline and candidate source identities;
2. admit a corpus of real, incomplete, malformed, macro-heavy, generated, and multi-crate projects;
3. replay identical edit and query sequences;
4. compare normalized semantic outputs;
5. compare invalidated query sets and bounded recomputation;
6. compare cancellation and timeout behavior;
7. record every divergence;
8. classify acceptable intentional differences explicitly;
9. issue an equivalence receipt only when the admission policy is satisfied;
10. move authority to the candidate only after the receipt is admitted.

### Required comparison surfaces

- syntax tree and parse errors;
- diagnostics;
- hover;
- goto definition/declaration/type-definition/implementation;
- references;
- completion and resolve;
- code actions and assists;
- rename and prepare-rename;
- semantic tokens;
- inlay hints;
- document/workspace symbols;
- call/type hierarchy;
- formatting and range formatting;
- crate graph and project status;
- proc-macro and build-script results;
- cancellation;
- incremental invalidation;
- latency and memory bounds.

## Migration phases

### Phase 0: immutable upstream mirror

- pin rust-analyzer source SHA;
- define allowed facade dependencies;
- forbid edits to the mirrored baseline;
- bind baseline source identity into every differential receipt.

### Phase 1: `lsp-max` runtime shell

- replace the rust-analyzer transport/runtime shell;
- preserve the semantic engine behind the facade;
- implement initialize/shutdown/exit, document synchronization, cancellation, progress, and diagnostics;
- prove no semantic crate depends on LSP transport types.

### Phase 2: project admission and BRCE

- broker Cargo, rustc, rustfmt, flycheck, build scripts, and proc macros;
- add toolchain and environment admission;
- add typed timeout, unsupported, and drift refusals;
- prove zero unreceipted process launch.

### Phase 3: evidence and replay

- bind semantic revisions and diagnostics to project identity;
- persist receipt DAGs;
- replay document events and tool consequences;
- export LSIF and ontology projections from admitted evidence.

### Phase 4: semantic component replacement

Replace components only behind differential gates, beginning with the lowest-coupling surfaces:

1. protocol mapping and capability construction;
2. source-root and file-set admission;
3. syntax diagnostics and structural symbols;
4. project status and tool diagnostics;
5. lexical search and indexing;
6. assists and edit construction;
7. name resolution;
8. macro expansion;
9. type inference;
10. completion ranking and semantic UX parity.

### Phase 5: baseline retirement

The pinned rust-analyzer baseline may be removed only after every retained semantic surface has an admitted replacement receipt and the corpus replay remains green without the baseline implementation.

## Executable first slice

The first runnable slice must do all of the following:

1. accept `initialize`, `shutdown`, `didOpen`, `didChange`, and `didClose` through `lsp-max`;
2. admit one bounded Rust project and its exact file set;
3. parse real Rust syntax and produce diagnostics;
4. create a semantic revision identity;
5. construct one source edit without mutation authority;
6. actuate that edit only through BRCE;
7. run one real Rust tool through BRCE;
8. bind authorization and consequence receipts;
9. replay the edit and tool result;
10. compare the candidate structural result with its pinned baseline;
11. expose machine-readable standing.

## Implemented 80/20 extension

The executable slice now also provides a bounded, content-addressed lexical workspace index and the standard editor features that deliver most day-to-day LSP value without pretending to reproduce rustc semantics:

- hover with item signatures;
- goto definition;
- references with optional declaration inclusion;
- prefix completion from admitted symbols and Rust keywords;
- document symbols;
- workspace symbols;
- prepare rename;
- multi-document rename construction;
- `max/raSnapshot` with index identity, verification, receipts, and explicit `lexical_only` standing.

The implementation excludes identifiers inside comments and strings, refuses duplicate definitions as typed ambiguity, refuses invalid rename targets, binds each index to project and semantic revision identity, and receipts rename plans as construction rather than actuation.

This surface has an exact JSON-RPC acceptance test that drives `initialize`, `didOpen`, hover, definition, completion, rename, and `max/raSnapshot` through `lsp-max::LspService`.

## Acceptance contract

The following gates are cumulative. Later phases do not replace earlier proof.

### Source identity

- exact `lsp-max` base SHA recorded;
- exact rust-analyzer baseline SHA recorded;
- candidate SHA recorded;
- no silent baseline movement.

### Runtime

- initialization lifecycle passes through `lsp-max`;
- cancellation reaches semantic queries and brokered tools;
- transport shutdown does not leak child processes;
- unsupported methods retain typed protocol behavior.

### Authority

- no direct `Command::spawn` outside BRCE;
- no direct filesystem mutation outside BRCE;
- no client `applyEdit` outside BRCE;
- construction tests prove no state mutation;
- negative tests prove stale or unauthorized intents are refused.

### Semantics

- real Rust parser executes;
- incomplete projects retain partial results;
- semantic revisions are deterministic;
- differential corpus produces explainable results;
- incremental invalidation stays within admitted bounds.

### Evidence

- authorization precedes every actuation;
- consequence follows observed execution;
- receipt chains verify;
- replay reconstructs the tested result;
- machine-readable standing distinguishes ALIVE, PARTIAL_ALIVE, BLOCKED, BUILD_BROKEN, and UNSUPPORTED.

### Performance

- keystroke-path receipts remain bounded and nonblocking;
- cold evidence export is asynchronous to semantic response construction;
- latency and memory regressions are compared to the pinned baseline;
- a candidate cannot replace the baseline solely because functional output matches.

## Architectural falsifiers

This RFC is falsified if any of the following is observed:

1. a semantic query directly launches a process;
2. a constructed edit mutates a file or client without BRCE;
3. a tool result enters Salsa ground state without admission identity;
4. a receipt is emitted without binding the exact project and semantic revision;
5. a project with recoverable source errors is collapsed into total runtime failure;
6. a component is replaced without exact-subject differential evidence;
7. a branch or release label substitutes for a source commit identity;
8. cold ontology or evidence projection blocks the interactive query path by default;
9. diagnostics are represented as proof of runtime ANDON standing;
10. a successful client response is treated as proof of observed document mutation;
11. baseline and candidate execute under different unrecorded toolchains or configurations;
12. duplicate lexical definitions are guessed into a semantic scope instead of refused;
13. identifiers in comments or strings are reported as source references;
14. rename construction is treated as edit actuation;
15. an LSP capability is claimed without execution through the JSON-RPC router.

## Explicit exclusions

The initial implementation does not claim:

- a from-scratch replacement of rustc semantics;
- immediate removal of Salsa;
- immediate replacement of rust-analyzer's parser or HIR;
- semantic equivalence based on unit tests alone;
- production parity after the first vertical slice;
- that every rust-analyzer subprocess is safe merely because it already exists;
- that LSIF or RDF projection belongs on the keystroke hot path;
- HIR, type-inference, macro-expansion, Cargo project-model, or performance parity from the lexical 80/20 surface.

## Consequences

### Positive

- runtime authority becomes explicit and auditable;
- rust-analyzer semantics can be preserved while infrastructure changes;
- semantic replacement becomes incremental instead of all-or-nothing;
- failures retain typed standing;
- source edits and external tools gain replayable consequence evidence;
- mature compiler intelligence can be integrated without granting it ambient actuation authority;
- the highest-value editor navigation and refactoring surfaces are available before HIR parity.

### Costs

- project and semantic identities must be propagated through every query and receipt;
- differential infrastructure is substantial;
- proc-macro and build-script isolation require platform-specific work;
- preserving interactive latency while recording evidence requires explicit hot/cold topology;
- dual-running baseline and candidate components increases temporary resource cost;
- lexical-only features must expose their boundary and refuse ambiguity instead of silently approximating Rust scope.

## Operational standing

At exact validated head `f277998fc72dae16cf928c6cb01e30c032ec30ad`:

- project admission, structural Rust parsing, semantic revision identity, BLAKE3 receipt chains, brokered source edits, brokered real `rustc` execution, differential replay, and JSON demo are ALIVE;
- hover, definition, references, completion, document/workspace symbols, prepare rename, and rename construction are ALIVE for admitted unique lexical symbols;
- the JSON-RPC route from `LspService` through those handlers is ALIVE;
- duplicate-definition scope resolution is explicitly UNSUPPORTED and produces typed ambiguity;
- rust-analyzer HIR, macro expansion, type inference, Cargo project modeling, semantic completion parity, incremental-performance parity, and production replacement standing remain UNSUPPORTED.

RFC status remains **Proposed** because an implemented vertical slice does not establish full rust-analyzer replacement standing.
