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

Start with the Coordinator initial snapshot.

