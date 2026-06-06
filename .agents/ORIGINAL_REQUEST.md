# Original User Request

## Initial Request — 2026-06-04T17:10:49-07:00

Convert the downloaded `tower-lsp-max-specgen` scaffold into a Rust workspace layout at `~/tower-lsp-max` while preserving existing workspace crates (`tower-lsp-max-macros`, `tower-lsp-max-protocol`, `tower-lsp-max-runtime`, `tower-lsp-max-agent`).

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Copy and Organize Source Crate
- Copy the `tower-lsp-max-specgen` source crate from `~/Downloads/tower-lsp-max-specgen` to `/Users/sac/tower-lsp-max/crates/tower-lsp-max-specgen`.
- Rationale: Structure the workspace cleanly without affecting the existing root level crates.

### R2. Workspace Initialization and Cargo/Git Setup
- Update `/Users/sac/tower-lsp-max/Cargo.toml` to include `"crates/tower-lsp-max-specgen"` in the workspace members.
- The workspace members list must look like:
  ```toml
  [workspace]
  members = [
      ".",
      "./tower-lsp-max-macros",
      "./tower-lsp-max-protocol",
      "./tower-lsp-max-runtime",
      "./tower-lsp-max-agent",
      "crates/tower-lsp-max-specgen",
  ]
  ```
- Ensure workspace lint rules, package metadata, and edition are correctly set.
- Ensure Git is initialized and `.gitignore` matches required files (e.g., target, generated/ files, and logs).

### R3. Setup Documentation and Architecture Guidelines
- Create ADR document `docs/adr/ADR-0001-tower-lsp-max-purpose.md` explaining decision to bootstrap generator first.
- Create system framework guide `docs/law/law-state-protocol-frame.md` explaining planned design space (protocol, server, runtime, law plugins).
- Write `docs/reports/SPECGEN-001-bootstrap-report.md` capturing environment, file list, verification command output, and next steps.

### R4. Verification and Sample Generation
- Ensure workspace formatting and type correctness (`cargo fmt --check`, `cargo check --workspace`, `cargo test --workspace`).
- Run the generator to produce `generated/lsp_minimal.rs` from `crates/tower-lsp-max-specgen/fixtures/minimal-metaModel.json`.

## Acceptance Criteria

### Project Verification and Compilation
- [ ] `cargo check --workspace` compiles successfully with no workspace check errors.
- [ ] `cargo fmt --check` passes successfully.
- [ ] `cargo test --workspace` executes successfully.
- [ ] Generator command `cargo run -p tower-lsp-max-specgen -- --input crates/tower-lsp-max-specgen/fixtures/minimal-metaModel.json --output generated/lsp_minimal.rs` exits successfully.
- [ ] File `generated/lsp_minimal.rs` exists, and its top lines show valid Rust code (inspected via standard CLI or reading).
- [ ] Artifact `docs/reports/SPECGEN-001-bootstrap-report.md` exists and contains the requested status and commands table.

## Follow-up — 2026-06-04T18:34:35-07:00

# MISSION

You are a 10-agent post-human framework engineering team operating inside:

  /Users/sac/tower-lsp-max

Your job is NOT to create another pile of features.

Your job is to convert `tower-lsp-max` into a cleanly bounded, machine-verifiable, post-human LSP substrate.

The repository has already had major work performed:

  - LSP v3.18.0 research and generation were attempted.
  - Generated LSP 3.18 vocabulary appears to have been integrated.
  - Custom max/* methods were reportedly added.
  - CLI noun modules were reportedly implemented.
  - Workspace tests were reportedly passing at 48 tests.
  - Prior agent runs caused high concurrency, cargo lock contention, and unmanaged subagent buildup.

Treat those reports as claims, not truth.

Your first duty is to calculate the repository state.

This is not a human-review workflow.

This is a conformance calculation workflow.

# CENTRAL DOCTRINE

tower-lsp-max is not “tower-lsp with more editor features.”

tower-lsp-max is:

  LSP as maximal protocol projection surface
  for machine-readable project state,
  generated protocol vocabulary,
  capability vectors,
  law-state diagnostics,
  transactional repair plans,
  conformance vectors,
  analysis bundles,
  receipt-bearing server operations,
  and future agent/CI/generator consumption.

Do not optimize for human onboarding.

Do not write toy tutorials.

Do not explain this as an IDE helper.

The primary consumers are:

  - agents
  - generators
  - release gates
  - CI
  - law-state servers
  - framework conformance calculators
  - future w4pm/ggen/unrdf LSP-Max domain plugins

# ABSOLUTE CONCURRENCY LAW

Only one agent may run global cargo commands.

The Verifier Agent owns:

  cargo fmt --check
  cargo check --workspace
  cargo test --workspace
  cargo clippy --workspace --all-targets -- -D warnings

No other agent may run those commands unless the Verifier explicitly delegates.

Reason:

  Prior runs created cargo lock contention and process-kill chaos.
  This workflow must be deterministic.

Agents may inspect files, write reports, and propose changes.
Agents may run targeted non-cargo commands.
Agents may run cargo only in a narrow package if explicitly assigned by the Coordinator and if the Verifier is idle.

No agent may kill cargo processes unless the Coordinator declares BLOCKED_CARGO_LOCK and records the reason.

# STATUS VOCABULARY

Use only these status terms:

  MAX_AUDIT_COMPLETE
  MAX_IMPLEMENTATION_PARTIAL
  MAX_IMPLEMENTATION_COMPLETE
  BLOCKED_REPO_DIRTY
  BLOCKED_CARGO_LOCK
  BLOCKED_SPECGEN
  BLOCKED_PROTOCOL_SURFACE
  BLOCKED_RUNTIME_LAW
  BLOCKED_CLI_SURFACE
  BLOCKED_TEST_FAILURE
  BLOCKED_UNKNOWN

Do not say:

  looks good
  probably fine
  human review required
  should be okay
  ship it

# GLOBAL DELIVERABLE

Produce:

  docs/reports/MAX-001-ten-agent-conformance-report.md

This report must include:

  - repo snapshot
  - git status
  - current crate layout
  - generated protocol status
  - custom max/* method status
  - CLI noun status
  - runtime law-state status
  - missing gates
  - verification command results
  - exact BLOCKED or COMPLETE status
  - next gates MAX-002 through MAX-005

# NON-GOALS

Do not publish anything.
Do not push to remote.
Do not create crates.io releases.
Do not fork upstream tower-lsp yet.
Do not add wasm4pm-specific diagnostics yet.
Do not add ggen-specific diagnostics yet.
Do not add unrdf-specific diagnostics yet.
Do not create toy tutorials.
Do not add “getting started” docs.
Do not overclaim protocol completeness.

# REQUIRED INITIAL INSPECTION

The Coordinator must begin with:

  cd /Users/sac/tower-lsp-max
  pwd
  git status --short
  git log --oneline -5
  find . -maxdepth 3 -type f | sort | sed 's#^\./##' | head -300
  find crates -maxdepth 3 -type f | sort
  find docs -maxdepth 4 -type f | sort || true

Then record:

  docs/reports/MAX-001-initial-snapshot.md

Do not let implementation agents begin edits until this snapshot exists.

# TEAM STRUCTURE

Define and launch exactly 10 agents.

No more.

If the system already has active stale subagents, the Coordinator must list them first.
Do not stack unlimited agents on top of stale ones.

The 10 agents are:

  1. max_coordinator
  2. specgen_metamodel_agent
  3. generated_protocol_agent
  4. lsp_surface_comparator_agent
  5. max_protocol_agent
  6. law_state_runtime_agent
  7. transaction_repair_agent
  8. cli_surface_agent
  9. docs_law_agent
  10. verifier_agent

Each agent must write a report under:

  docs/reports/agents/MAX-001-<agent-name>.md

The Coordinator composes the final report.

# AGENT 1 — max_coordinator

## Role

You are the workflow governor.

You own sequencing, conflict control, and final conformance status.

## Inputs

  /Users/sac/tower-lsp-max

## Responsibilities

1. Create report directory:

  docs/reports/agents/

2. Record initial repo state.

3. Define the exact work queue.

4. Prevent overlapping cargo commands.

5. Assign file ownership.

6. Merge findings from all agents.

7. Produce final report:

  docs/reports/MAX-001-ten-agent-conformance-report.md

## File ownership

You may edit only:

  docs/reports/MAX-001-initial-snapshot.md
  docs/reports/MAX-001-ten-agent-conformance-report.md

unless resolving report conflicts.

## Required report sections

  - Current git HEAD
  - Dirty files before work
  - Existing crates
  - Existing generated files
  - Existing docs
  - Agent assignments
  - Cargo command lock policy
  - Final status

# AGENT 2 — specgen_metamodel_agent

## Role

You own the official LSP meta-model ingestion and generator correctness.

## Questions to answer

  - Is the official LSP 3.18 metaModel fixture present?
  - Is it named clearly?
  - Does the generator parse it?
  - Does the generator model all meta-model type variants?
  - Are complex forms handled explicitly or collapsed silently?
  - Are literal, or, and, tuple forms tracked as known law?

## Inspect

  crates/tower-lsp-max-specgen/
  crates/tower-lsp-max-specgen/src/main.rs
  crates/tower-lsp-max-specgen/src/metamodel.rs
  crates/tower-lsp-max-specgen/src/render.rs
  crates/tower-lsp-max-specgen/fixtures/

## Required output

  docs/reports/agents/MAX-001-specgen-metamodel-agent.md

## Required report structure

  # MAX-001 Specgen Metamodel Agent Report

  ## Status
  MAX_IMPLEMENTATION_COMPLETE
  or exact BLOCKED_* status

  ## MetaModel Fixtures

  ## Supported Type Kinds

  ## Unsupported / Conservative Lowerings

  ## Generator Commands

  ## Required Follow-up Gates

## Edit policy

You may edit:

  crates/tower-lsp-max-specgen/src/metamodel.rs
  crates/tower-lsp-max-specgen/src/render.rs
  crates/tower-lsp-max-specgen/src/main.rs
  crates/tower-lsp-max-specgen/fixtures/
  docs/reports/agents/MAX-001-specgen-metamodel-agent.md

Do not edit protocol/server/runtime crates.

# AGENT 3 — generated_protocol_agent

## Role

You own generated Rust protocol vocabulary hygiene.

## Questions to answer

  - Where is the generated LSP 3.18 Rust surface?
  - Is it committed source, generated artifact, or build output?
  - Is there a stable module exposing it?
  - Does generated output contain serde derives?
  - Does generated output use LspAny / serde_json::Value intentionally?
  - Are recursive or self-referential structures handled safely?
  - Are numeric enums serialized/deserialized correctly?
  - Are generated names stable?

## Inspect

  generated/
  src/
  crates/
  any generated_3_18.rs
  any lsp_3_18.rs
  Cargo.toml files

## Required output

  docs/reports/agents/MAX-001-generated-protocol-agent.md

## Edit policy

You may edit:

  generated/
  src/generated_3_18.rs
  src/lsp_3_18.rs
  crates/*/src/generated*.rs
  crates/*/src/lsp_3_18.rs
  docs/reports/agents/MAX-001-generated-protocol-agent.md

Do not change server behavior.

## Hard law

If generated Rust is checked in, document why.
If generated Rust is ignored, document how clients consume it.
No hidden generated boundary.

# AGENT 4 — lsp_surface_comparator_agent

## Role

You compare tower-lsp-max protocol coverage against the modern LSP surface.

This is not a web research task unless local docs are missing.
Use the already downloaded LSP meta-model fixture first.

## Questions to answer

  - What requests exist in the meta-model?
  - What notifications exist?
  - What structures exist?
  - What enumerations exist?
  - What type aliases exist?
  - Which are represented in generated Rust?
  - Which are routed in server code?
  - Which are exposed only as types but not handlers?
  - Which are intentionally unsupported?

## Required output

  docs/reports/agents/MAX-001-lsp-surface-comparator-agent.md

## Optional generated artifact

  docs/reports/LSP-3.18-SURFACE-COMPARISON.md

## Edit policy

You may edit only docs/reports files unless the Coordinator assigns a specific code fix.

## Critical distinction

Do not confuse:

  protocol vocabulary coverage

with:

  server implementation coverage

Types existing does not mean methods are implemented.

# AGENT 5 — max_protocol_agent

## Role

You own the custom `max/*` protocol surface.

## Required conceptual model

The max protocol must expose post-human law-state operations, such as:

  max/snapshot
  max/conformanceVector
  max/explainDiagnostic
  max/repairPlan
  max/applyRepairTransaction
  max/exportAnalysisBundle
  max/runGate
  max/clearDiagnostic
  max/receipt

## Questions to answer

  - Which max/* methods exist?
  - Are they typed?
  - Are they routed?
  - Are they tested?
  - Are responses deterministic?
  - Are errors structured?
  - Do they produce analysis-bundle-compatible records?
  - Are receipt claims real or placeholder?

## Inspect

  src/lib.rs
  src/service.rs
  src/service/
  any protocol or runtime crates
  tests/

## Required output

  docs/reports/agents/MAX-001-max-protocol-agent.md

## Edit policy

You may edit:

  src/lib.rs
  src/service.rs
  src/service/
  tests/
  docs/reports/agents/MAX-001-max-protocol-agent.md

Do not edit CLI nouns.

## Hard law

If a receipt is only hash-shaped and not cryptographically complete, call it:

  structural receipt

not:

  cryptographically sound receipt

No false cryptographic claims.

# AGENT 6 — law_state_runtime_agent

## Role

You own the law-state runtime model.

This is the heart of tower-lsp-max.

## Required primitives

Check whether the codebase has, or needs, these abstractions:

  SnapshotId
  CapabilityVector
  MaxDiagnostic
  LawId
  TransitionAttempt
  LawAxis
  RepairAction
  ValidationPlan
  RollbackPlan
  ReceiptObligation
  AnalysisBundle
  ConformanceVector
  ConformanceStatus

## Questions to answer

  - Are these modeled as real Rust types?
  - Are they scattered or centralized?
  - Is there a runtime crate?
  - Is the law-state runtime independent from LSP transport?
  - Are diagnostics merely strings or structured refused transitions?
  - Are snapshots deterministic?
  - Is Unknown distinct from Refused?
  - Is Admitted distinct from Passed?

## Required output

  docs/reports/agents/MAX-001-law-state-runtime-agent.md

## Edit policy

You may edit:

  tower-lsp-max-runtime/
  crates/tower-lsp-max-runtime/
  src/runtime/
  src/lib.rs if runtime is currently embedded there
  docs/reports/agents/MAX-001-law-state-runtime-agent.md

Coordinate with max_protocol_agent before editing shared files.

## Hard law

Do not implement domain-specific wasm4pm/ggen rules here.

The runtime is generic.

# AGENT 7 — transaction_repair_agent

## Role

You own transactional code actions and repair plans.

## Required model

A Max repair is not a quick fix.

It is:

  preconditions
  workspace edit
  validation plan
  rollback plan
  diagnostic delta
  receipt plan

## Questions to answer

  - Are code actions currently plain LSP CodeAction values?
  - Is there a MaxCodeAction wrapper?
  - Can a repair be previewed?
  - Can a repair be applied transactionally?
  - Can a repair be rolled back?
  - Can a repair require validation gates?
  - Can a repair produce an analysis bundle?

## Required output

  docs/reports/agents/MAX-001-transaction-repair-agent.md

## Edit policy

You may edit:

  src/lib.rs
  src/service.rs
  src/service/
  crates/*repair*
  crates/*runtime*
  tests/
  docs/reports/agents/MAX-001-transaction-repair-agent.md

Coordinate with max_protocol_agent and law_state_runtime_agent.

## Hard law

Do not call a workspace edit “safe” unless there is an explicit validation plan.

# AGENT 8 — cli_surface_agent

## Role

You own the CLI command surface.

The transcript claims the CLI modules were implemented across nouns including server, client, workspace, metamodel, diagnostics, plugin, config, state, telemetry, and agent.

Treat that as a claim.

Calculate truth.

## Questions to answer

  - Which CLI crate exists?
  - Which nouns exist?
  - Which verbs exist?
  - Which verbs are real versus placeholders?
  - Do command handlers obey low-complexity noun-verb law?
  - Do commands produce machine-readable output?
  - Are config writes deterministic?
  - Are server/client commands actually safe?
  - Are agent commands real or dangerous placeholders?

## Inspect

  crates/tower-lsp-max-cli/
  crates/tower-lsp-max-cli/src/main.rs
  crates/tower-lsp-max-cli/src/nouns/

## Required output

  docs/reports/agents/MAX-001-cli-surface-agent.md

## Edit policy

You may edit:

  crates/tower-lsp-max-cli/
  docs/reports/agents/MAX-001-cli-surface-agent.md

Do not edit core server runtime without coordination.

## Hard law

If a command only prints a message, classify it as:

  presentational

not:

  implemented

If a command changes files, it must declare output paths and side effects.

# AGENT 9 — docs_law_agent

## Role

You own docs-as-release-law for tower-lsp-max.

## Required docs

Ensure these exist or create them:

  docs/law/post-human-lsp-frame.md
  docs/law/max-protocol-law.md
  docs/law/law-state-runtime-primitives.md
  docs/law/no-human-review.md
  docs/adr/ADR-0001-tower-lsp-max-purpose.md
  docs/adr/ADR-0002-generated-protocol-vocabulary.md
  docs/reports/MAX-001-ten-agent-conformance-report.md

## Required doctrine

The docs must say:

  - LSP is a post-human project-state protocol.
  - tower-lsp-max is not an IDE helper.
  - human review is not a correctness gate.
  - correctness is a conformance calculation.
  - diagnostics are refused transitions.
  - code actions are repair transactions.
  - docs are law projections, not onboarding tutorials.
  - generated protocol vocabulary is a source of protocol truth.

## Required output

  docs/reports/agents/MAX-001-docs-law-agent.md

## Edit policy

You may edit:

  docs/
  README.md
  CLAUDE.md if present
  AGENTS.md if present

Do not edit Rust source.

# AGENT 10 — verifier_agent

## Role

You own machine verification.

You are the only agent authorized to run global cargo commands.

## Required command sequence

After implementation agents finish, run:

  cd /Users/sac/tower-lsp-max
  git status --short
  cargo fmt --check
  cargo check --workspace
  cargo test --workspace
  cargo clippy --workspace --all-targets -- -D warnings

If clippy is too noisy due to existing baseline, record exact failure and classify as:

  BLOCKED_TEST_FAILURE

Do not paper over warnings.

## Additional checks

  find . -name '.DS_Store' -print
  find . -path '*/target/*' -prune -o -type f -print | sort | head -300
  git diff --stat
  git status --short

## Required output

  docs/reports/agents/MAX-001-verifier-agent.md

## Report format

  # MAX-001 Verifier Agent Report

  ## Status

  ## Commands

  | Command | Result | Notes |
  |---|---|---|

  ## Dirty Tree

  ## Forbidden Files

  ## Failing Gates

  ## Final Conformance Vector

## Hard law

You do not “review.”

You calculate.

# FILE OWNERSHIP COLLISION RULE

If two agents need the same file, the Coordinator decides ownership.

Shared high-risk files:

  src/lib.rs
  src/service.rs
  src/service/state.rs
  crates/tower-lsp-max-cli/src/nouns/*.rs
  Cargo.toml

No simultaneous edits to shared high-risk files.

# REQUIRED FINAL REPORT

The Coordinator must write:

  docs/reports/MAX-001-ten-agent-conformance-report.md

with this structure:

  # MAX-001 Ten-Agent Conformance Report

  ## Status
  MAX_IMPLEMENTATION_COMPLETE
  or exact BLOCKED_* status

  ## Repository Snapshot

  ## Agent Reports

  | Agent | Status | Report |
  |---|---|---|

  ## Current Architecture

  ## Protocol Coverage

  ## Runtime Law-State Coverage

  ## CLI Coverage

  ## Verification Commands

  ## Dirty Tree

  ## Known Limitations

  ## Next Gates

  ### MAX-002 — Protocol Vocabulary Closure
  Implement robust lowering for all LSP meta-model forms without silent serde_json::Value collapse.

  ### MAX-003 — Max Protocol Stabilization
  Stabilize max/* request and notification schemas.

  ### MAX-004 — Runtime Separation
  Split protocol, server, runtime, CLI, and domain-plugin surfaces.

  ### MAX-005 — Domain Plugin First Cell
  Add first non-toy domain plugin only after generic law-state substrate is stable.

# IMPLEMENTATION DISCIPLINE

Do not chase “all features.”

Every change must map to one of these law-state primitives:

  - protocol vocabulary
  - capability vector
  - deterministic snapshot
  - structured diagnostic
  - refused transition
  - repair transaction
  - conformance vector
  - analysis bundle
  - receipt obligation
  - CLI actuation surface

If a change does not map to one of those, refuse it.

# EXECUTION ORDER

1. Coordinator creates snapshot and reports directory.
2. Specgen, generated protocol, comparator, CLI, docs agents inspect in parallel.
3. Runtime, protocol, and transaction agents coordinate edits after inspection.
4. Verifier runs global gates only after edits settle.
5. Coordinator writes final conformance report.

# FINAL OUTPUT TO USER

Return only:

  - final status
  - path to final report
  - command table summary
  - blocked gates if any
  - next recommended gate

Do not ask for human review.

Do not ask whether the code “looks good.”

The repository is either admitted by conformance calculation or refused with named failing constraints.

# BEGIN NOW



## Follow-up — 2026-06-04T21:27:15-07:00

Verify and close the protocol vocabulary generation logic, ensuring every LSP 3.18 meta-model type kind has an explicit Rust lowering policy in `tower-lsp-max-specgen` and passes all workspace checks.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: development

## Requirements

### R1. Lowering Policy Mapping & Audit
- Analyze all types, structures, type aliases, enums, unions, and intersections in the LSP 3.18 meta-model.
- Document an explicit lowering policy for each category:
  - Native Rust type
  - Boxed recursive type
  - Transparent newtype
  - Tagged/untagged enum
  - Intentional `LspAny` / `serde_json::Value` fallback
  - Refused/unsupported form
- Save this analysis in a comprehensive lowering report.

### R2. Specgen Alignment
- Update the generator logic in `crates/tower-lsp-max-specgen` to enforce these explicit mapping policies.
- Ensure that any silent or undocumented fallback conversions are replaced by explicit structural lowerings or documented fallback policies.
- Regenerate the protocol types if generator policies are modified, and ensure compilation integrity.

### R3. Workspace Verification
- Run full verification checks on the entire workspace.
- The workspace must build cleanly with no formatting violations, no test failures, and no Clippy warnings under `-D warnings`.

## Acceptance Criteria

### Verification and Conformance
- [ ] The generated report `docs/reports/MAX-002-lowering-conformance.md` exists and details the explicit lowering policy for each meta-model type variant.
- [ ] `cargo check --workspace` compiles successfully.
- [ ] `cargo test --workspace` executes successfully.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes with zero warnings or errors.
- [ ] `cargo fmt --check` passes successfully.

## Follow-up — 2026-06-04T21:35:31-07:00

Implement the 5-layer Autonomic Manufactured Intelligence (AMI) mesh and Knowledge Hook Layer in `tower-lsp-max`, enabling chained LSP instances to coordinate state transitions, diagnostics, and repairs self-regulated by the A = μ(O*) equation.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Formal LSP Representation & Hook Layer
- Implement the formal LSP model representation: `LSP_i = ⟨O_i*, H_i, Φ_i, D_i, R_i, A_i, ρ_i⟩`.
- Define a reusable `Hook` / `H_i` interface representing state-triggered knowledge/action routing.
- Enable hooks to be registered for specific state changes, diagnostics (`D_i`), repair plans (`R_i`), or receipts (`ρ_i`).

### R2. 5-Layer Autonomic Architecture
- **Actuation Grammar**: CLI command parser linking noun/verbs and `#[verb]` actions.
- **Local LSP Surface**: JSON-RPC layer exposing diagnostics, code actions, workspace edits, and custom `max/*` methods.
- **Law-State Runtime**: Transition validation, compliance evaluation `Φ(O*)`, and emission of receipts (`ρ`) or diagnostics (`D`).
- **Knowledge Hook Layer**: Event routing routing events (diagnostics, receipts) to hook interfaces.
- **Autonomic LSP Mesh**: Mesh controller running multiple chained LSPs where consequence/events of one trigger hooks of another.

### R3. Customer Service Proof Case
- Realize the customer service flow as a verification proof case:
  `customer language → old-AI parse/classify → missing-state diagnostics → clarifying question → policy/process transition → bounded action → receipt → knowledge hook`
- Show how a diagnostic event in `LSP_1` hooks into `LSP_2` to synthesize a repair plan, update state, and emit a receipt.

### R4. Workspace Verification
- Ensure the entire workspace compiles, format-checks, and tests cleanly.

## Acceptance Criteria

### Execution & Integration
- [ ] A multi-LSP integration test (`tests/test_autonomic_mesh.rs`) demonstrates that a diagnostic or receipt in `LSP_1` triggers a hook in `LSP_2`, generating a repair plan and updating its state.
- [ ] `cargo check --workspace` compiles successfully.
- [ ] `cargo test --workspace` executes successfully.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes with zero warnings.
- [ ] `cargo fmt --check` passes successfully.

## Follow-up — 2026-06-05T17:58:13Z

Refactor all Rust files in the workspace containing more than 500 lines of code (LOC) into smaller, modular files, following Rust idiomatic design patterns, without breaking any compilation or test functionality.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Refactor Files Exceeding 500 LOC
Identify all Rust files (`*.rs`) in the repository (excluding `target/` and `.git/` directories) that contain more than 500 lines of code (as counted by `wc -l`). Refactor these files by splitting their contents into smaller, logically coherent modules or files.

### R2. Maintain Rust Idioms and Visibility
Use standard Rust modular organization (e.g., creating a module folder with `mod.rs` or a sub-module file structure). Maintain correct visibility (`pub`, `pub(crate)`) and keep imports (`use` statements) clean.

### R3. Preservation of Functionality
The refactoring must not alter any public behavior or features of the codebase. The workspace must compile successfully and all tests must pass.

## Verification Mechanisms

To verify that the requirements have been successfully met, the following steps must be run:
1. **Line Count Verification**: Verify that no Rust file in the repository (excluding `target/`, `.git/`, and `.claude/`) has more than 500 lines. This can be verified by running:
   ```bash
   find . -name "*.rs" -not -path "*/target/*" -not -path "*/.claude/*" | xargs wc -l | sort -rn
   ```
   and ensuring the highest line count for any individual file is <= 500.
2. **Compilation Verification**: Run `cargo check --all-targets` and `cargo build --all-targets` to verify successful compilation with no new errors.
3. **Behavioral Verification**: Run `cargo test --all-targets` to verify all tests pass.

## Acceptance Criteria

### Restructured Codebase
- [ ] No `.rs` file in the codebase (excluding `target/` and `.claude/` directories) exceeds 500 lines of code.

### Correctness
- [ ] `cargo check --all-targets` completes with exit code 0.
- [ ] `cargo build --all-targets` completes with exit code 0.
- [ ] `cargo test --all-targets` completes with exit code 0.

## Follow-up — 2026-06-05T18:37:09Z

Avoid sequential or suffix-based file naming (such as `_part1.rs`, `_part2.rs`); name split module files based on their logical categorization or functionality (e.g. `metadata.rs`, `resolution.rs`). Rename any existing files that use poor naming patterns.

## Follow-up — 2026-06-05T21:52:19Z

Implement the Oxigraph/SPARQL Admitted Graph Control Plane and Ostar Generative Pipeline integration in tower-lsp-max, as specified in the PRD/ARD documents inside `docs/reports/` and `docs/v26.6.5/prd-ard/`.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Admitted RDF Graph State & Oxigraph Integration
- Implement the `RelationAdmitter` trait supporting states: `RAW`, `CANDIDATE`, `ADMITTED`, `REFUSED`, `QUARANTINED`, `SUPERSEDED`, `REPLAYED`.
- Support both in-memory `oxigraph::store::Store` (default) and persistent RocksDB-backed `Store` (via a configurable storage path).
- Successfully translate LSIF 0.6.0 elements (documents, ranges, vertices, edges, item properties) and diagnostic observations into `oxrdf::Quad` triples using standard vocabularies (LSIF, PROV-O, DCTERMS, etc.).

### R2. SPARQL Invariant Verification & Diagnostics
- Enforce the 5 Core Invariants:
  1. *No orphan LSIF relations*: Validate that all LSIF edge targets point to existing vertices using SPARQL `ASK`.
  2. *No unreceipted graph consequences*: Every diagnostic or protocol artifact must have a `prov:wasGeneratedBy` receipt link.
  3. *No hot-path SPARQL dependency*: Ensure interactive LSP query loops do not execute SPARQL queries directly.
  4. *No ontology laundering*: Private terms (`max:`) must not masquerade as public semantics.
  5. *No false ALIVE*: Valid status requires successful cryptographic replay verification.
- Capture and report structural errors as detailed `VerificationReport` diagnostics, refusing invalid fixtures.

### R3. Materialized View & LSP Routing
- Implement asynchronous materialized views (e.g. using `DashMap` or structured indexes) populated by background SPARQL queries.
- Serve standard LSP lookup requests (`textDocument/definition`, `textDocument/references`, `textDocument/hover`, and `textDocument/publishDiagnostics`) directly from these materialized views in `O(1)` time.

### R4. Cryptographic Receipt Chaining
- Implement a robust `CryptographicReceipt` structure in Rust (and a key management mechanism for Ed25519 signing) that records transition metadata: `prev_hash`, `discipline_id`, `law_id`, `consequence_hash`, and `sequence`.
- Compute and chain digests using BLAKE3 to build an immutable, chronological execution chain.

### R5. Deterministic Replay Engine
- Implement a query consequence replay verifier.
- Re-run transitions in isolation: initialize states from genesis parameters in the trace log, mock/stub system clocks and random seeds deterministically, and assert that recomputed state hashes match the signed receipts.

### R6. Ostar Typestate Kernel Integration
- Bind the codebase transitions to the generic `Machine<L, P, D>` container and compile-time checked `TypestateKernel` trait.
- Enforce linear consumption of states using Rust's affine ownership type system (`self` moves).

## Acceptance Criteria

### Compilation & Tests
- [ ] All code compiles cleanly under `cargo check` and contains no warnings under `cargo clippy`.
- [ ] `cargo test` passes 100% across the workspace, including new unit and integration tests for the admitted graph, SPARQL queries, materialized views, receipt chaining, and deterministic replay.
- [ ] Existing LSIF parser baseline tests remain green without regression.

### Graph Admission & Query Verification
- [ ] A sample LSIF fixture is successfully parsed, admitted into the `oxigraph::Store`, and validated against the 5 invariants.
- [ ] Malformed/invalid graph fixtures are successfully detected, quarantined, and refused with corresponding diagnostic explanations.

### Hot-Path Views & Replay
- [ ] LSP queries (e.g. Definition) resolve from the materialized views without calling the Oxigraph store.
- [ ] The replay engine successfully runs a verification against a generated receipt chain, producing a matching cryptographic digest and proving replay determinism.
- [ ] No stubs, placeholders, `TODO`s, or unimplemented sections remain in the active codebase.

## Follow-up — 2026-06-05T22:22:27Z

Implement a production-grade 'ALIVE' release candidate for tower-lsp-max v26.6.5, integrating the Oxigraph & SPARQL Admitted Graph Control Plane and completing the remaining planned milestones (M3–M7) for library modularization.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Admitted RDF Graph State
Ingest workspace files, ranges, LSIF constructs (vertices/edges), LiveLSP diagnostics, and receipts into an embedded Oxigraph (v0.5.8) database as RDF triples. Enforce strict namespace mapping using standard prefixes (`rdf:`, `rdfs:`, `prov:`, `lsif:`) and bounded private prefixes (`max:`, `rcpt:`).

### R2. SPARQL Invariant & SHACL Shapes Gate
Enforce SHACL shape constraints on ingested triples to reject structurally malformed data. Run 5 core SPARQL validation queries (ASK/SELECT) at transaction commit to check for orphans, unregistered namespaces, unreceipted diagnostics, and lack of projections, blocking snapshot updates if any invariant is violated.

### R3. Materialized Views & Epoch Sync Barrier
Decouple live LSP definition, references, and hover requests from SPARQL execution by projecting query results asynchronously into in-memory `DashMap` structures to keep hot-path latencies below 5ms. To prevent race conditions, implement a Monotonic Epoch Sync Barrier that blocks strict-accuracy read requests (from agents/verifiers) when `committed_epoch > applied_epoch` until projection sync completes.

### R4. Cryptographic Receipt Functor
Ensure every admitted diagnostic or projection produces a BLAKE3 cryptographic receipt functor (`max:Receipt`) linking the input graph, query, and result hashes. Maintain functoriality ($\rho(g \circ f) = \rho(g) \circ \rho(f)$) and verify replay determinism by checking that replay query outputs match the receipt's expected result hash.

### R5. Protocol Projection Surface
Provide projection interfaces transforming the admitted graph and materialized views into standard JSON-RPC LSP, LSIF 0.6.0 NDJSON exports, Model Context Protocol (MCP) tool/resource lists, and Agent-to-Agent (A2A) task/agent capability cards.

### R6. Workspace Refactoring & Decoupling (M3–M7)
Refactor and modularize `tower-lsp-max-protocol/src/lib.rs`, `tower-lsp-max-runtime/src/lib.rs`, and the core server modules (`src/lib.rs`, `src/service.rs`, `src/service/client.rs`), splitting large inline files so that all primary source files are under 500 lines of code (LOC).

## Verification Resources
- Use the existing tests inside `tests/` directory (e.g., `test_rocksdb_admission.rs`, `test_materialized_views_integration.rs`, `test_challenger_m3_verification.rs`) as a reference verification harness.
- Add comprehensive integration and unit tests for the Oxigraph control plane, SPARQL invariants, materialized views, and BLAKE3 receipts.

## Acceptance Criteria

### Build & Quality Gates
- [ ] Workspace compiles cleanly on stable Rust channel.
- [ ] `cargo fmt --check` succeeds across all workspace crates.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes without warning/error.
- [ ] All tests in `cargo test --workspace` pass (minimum of 400+ passing tests).

### Functional Correctness
- [ ] Structurally invalid or laundered triples are rejected at the ingestion boundary.
- [ ] 5 core SPARQL invariants are correctly checked at transaction commit.
- [ ] Interactive definition lookups serve from in-memory materialized views with a latency under 5ms.
- [ ] Monotonic Epoch Sync Barrier blocks reads under write contention when strict accuracy is requested.
- [ ] All diagnostics/projections contain valid BLAKE3 receipts, and independent verifier replays confirm 100% hash determinism.

### Code Style & Decoupling
- [ ] Target refactored source files are modularized and stay under 500 LOC.
- [ ] All existing comments and docstrings unrelated to code changes are preserved.

## Follow-up — 2026-06-05T23:07:42Z

Verify all LSP, LSIF, and Base protocol implementations in the `tower-lsp-max` workspace against their official specifications: LSIF 0.6.0, LSP Base 0.9, and LSP 3.18. Address any gaps or non-conformances found, and implement comprehensive test cases to verify compliance.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. LSIF 0.6.0 Conformance Verification & Fixes
Audit and verify the `tower-lsp-max-lsif` crate against the LSIF 0.6.0 specification (https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/). Specifically, check that:
- Every vertex and edge label, property, and variant matches the specification (e.g. metadata, moniker, hover, document symbol, project, document, range).
- Serialization/deserialization behaves exactly as required by LSIF (NDJSON format).
- Address any gaps or deviations found.

### R2. LSP Base 0.9 Conformance Verification & Fixes
Audit and verify the base protocol types and abstractions in `crates/tower-lsp-max-base` and the core workspace against the LSP Base 0.9 specification (https://microsoft.github.io/language-server-protocol/specifications/base/0.9/specification/). Specifically, check that:
- JSON-RPC 2.0 envelopes, request/response headers, content-type, and content-length fields conform strictly.
- Content-Length parsing and boundary conditions are handled defensively and correctly.
- Address any gaps or deviations found.

### R3. LSP 3.18 Conformance Verification & Fixes
Audit and verify the protocol types, capabilities, and server lifecycle methods in `tower-lsp-max-protocol` and the core crate against the LSP 3.18 specification (https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/). Specifically, check that:
- Server and client capability structs correctly reflect all optional/required fields of LSP 3.18.
- Lifecycle method transitions (initialize, initialized, shutdown, exit) comply with the specification.
- Request, response, and notification serialization patterns (including untagged unions, selection ranges, call hierarchies, document symbols, and diagnostics) match the spec.
- Address any gaps or deviations found.

### R4. Programmatic Compliance Testing
Write programmatic test cases (unit and integration tests) to verify the compliance of:
- LSIF 0.6.0 vertex and edge serialization.
- LSP Base 0.9 headers, envelope structures, and parsing.
- LSP 3.18 key capability configurations, diagnostics, and workspace edits.

## Acceptance Criteria

### Verification & Test Suite
- [ ] Implement a test suite that programmatically validates conformance of the serialized JSON of LSP and LSIF structures against spec-expected structures.
- [ ] All workspace tests in `cargo test --workspace` pass cleanly.
- [ ] No compilation warnings or clippy warnings are introduced in the workspace.

### Code Quality & Modularization
- [ ] All new or refactored source files must remain under 500 lines of code (LOC).
- [ ] Existing code documentation and comments unrelated to code changes are preserved.

## Follow-up — 2026-06-05T23:13:04Z

Verify and complete the entire `tower-lsp-max` framework implementation using combinatorial maximalism. Resolve all compilation errors resulting from the recent LSIF struct changes, ensure absolute conformance with LSIF 0.6.0, LSP Base 0.9, and LSP 3.18 specifications, and implement rigorous verification via Oxigraph, SPARQL, and BLAKE3 receipts.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Fix Compilation & Align Structs
Resolve all compilation errors across the workspace (particularly in `tower-lsp-max-runtime/src/control_plane/kernel.rs` and `wasm4pm_graduation.rs`) caused by adding the `project_root` field to `Vertex::MetaData` and updating the `kind` field to `Option<String>` in `Vertex::Project`. Ensure all test fixtures and database initializations construct these types correctly.

### R2. LSIF 0.6.0 & LSP 3.18 Combinatorial Maximalism
Complete all vertex and edge typings according to the LSIF 0.6.0 specification, including new LSIF elements such as `CallHierarchyResult`, `TypeHierarchyResult`, `textDocument/callHierarchy`, and `textDocument/typeHierarchy`. Ensure the `oxigraph` mapping and SPARQL queries correctly process and validate all combinations of these structures.

### R3. SPARQL Invariants & SHACL Shape Gates
Ensure the SHACL-style property validation gates reject any malformed metadata, documents, and invalid severity values or line/character properties. Verify that the 5 core SPARQL invariants (orphans, unregistered namespaces, unreceipted diagnostics, lack of projections, false alive) block state transitions upon commit.

### R4. Caching Materialized Views & Epoch Sync Barrier
Ensure all definition, reference, hover, call/type hierarchy, and diagnostic requests are served from DashMap materialized views with latencies under 5ms. Validate that the Monotonic Epoch Sync Barrier blocks reads during write contention when strict accuracy is requested.

### R5. Cryptographic Receipts & Replay Verification
Ensure every state transition, diagnostic, or projection produces a BLAKE3 Merkle-DAG receipt functor (`max:Receipt`) linking inputs, queries, laws, and outcomes. Verify that functoriality and query replay determinism are programmatically proven.

### R6. Code Quality & Modularity (M3-M7)
Verify that all workspace crates (`tower-lsp-max`, `tower-lsp-max-protocol`, `tower-lsp-max-runtime`, `tower-lsp-max-base`, etc.) are modularized, compile cleanly, have zero warning/clippy issues, and keep every primary source file strictly under 500 lines of code (LOC).

## Acceptance Criteria

### Build & Compilation
- [ ] The entire workspace compiles cleanly without any errors or warnings.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes without issue.

### Verification & Correctness
- [ ] Add programmatic tests in the test suite that verify edge-case combinations of vertices and edges (including Call/Type hierarchies) and confirm they successfully round-trip and map to Oxigraph.
- [ ] 100% of the tests in `cargo test --workspace` pass successfully.

### Code Style & Decoupling
- [ ] Every source file is under 500 LOC (with unit tests extracted where necessary).
- [ ] Module-level and item-level docstrings are preserved.


