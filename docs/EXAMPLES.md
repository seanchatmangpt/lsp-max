# lsp-max Examples: Table of Contents

This document catalogs the 7 primary example LSP servers in the `examples/` directory. Each demonstrates distinct patterns and integrations over the lsp-max framework (LSP 3.18). All examples are workspace members and must keep compiling.

---

## 1. pattern-lsp

**Description:** Basic `LanguageServer` trait implementation using regex rule-based pattern detection on text documents. No external dependencies; custom routing via `clap-noun-verb` CLI grammar.

**Key Files:**
- `src/server.rs` — Core `PatternLsp` backend; implements `LanguageServer` trait for `did_open`, `did_change`, `did_close`, `did_save`
- `src/scanner.rs` — Regex pattern-matching logic; rule definitions and document scan
- `src/diagnostics.rs` — Diagnostic payload construction
- `src/rules.rs` — Rule registry and metadata

**Run:**
```bash
cargo run -p pattern-lsp -- serve --stdio
```

**What to Test:**
- Open a text file; pattern-lsp scans and publishes diagnostics in real time
- Modify the file; diagnostics update on change
- Close the file; diagnostics are cleared
- Test with sample `.txt` or `.log` files matching rule patterns (e.g., regex for error keywords)

**Expected Output:**
```
LSP initialize response → ServerCapabilities with textDocumentSync (FULL) and diagnosticProvider
Diagnostics published for each pattern match with rule ID, severity, and message
Clean shutdown on client disconnect
```

**README:** Not present; [create `pattern-lsp/README.md`](#creating-individual-readme-files).

---

## 2. anti-llm-cheat-lsp

**Description:** Law violation detection canary. Demonstrates enforcement of inverted LSP laws by detecting plain `tower-lsp` references, victory-claim language, unsafe code patterns, file mutation attempts, and unverified LSP 3.18 transcripts. Multi-layered detector stack with BLAKE3 receipt validation.

**Key Files:**
- `src/server.rs` — Core `AntiLlmServer` implementation; LSP 3.18 virtual documents (`anti-llm://failset`, `anti-llm://lsp318-matrix`, etc.)
- `src/engine.rs` — Main detection engine; rule evaluation, observation aggregation
- `src/config.rs` — Centralized victory vocabulary and law-axis definitions
- `src/ocel.rs` — OCEL trace export (process mining observation format)
- `src/parsers/` — Tree-Sitter AST scanning, Cargo.toml verification, Markdown claims parser
- `src/rules/` — Individual rule implementations (plain tower-lsp detector, unsafe code, file write attempts, victory language, version law)

**Run (LSP mode):**
```bash
cargo run -p anti-llm-cheat-lsp -- serve --stdio
```

**Run (Scan mode):**
```bash
cargo run -p anti-llm-cheat-lsp -- scan --dir /path/to/project [--ignore-dirs dir1 dir2]
```

**What to Test:**
- LSP: Publish diagnostics for `tower_lsp::` imports, `.unwrap()` patterns, victory words ("done", "fully admitted"), file mutations
- Scan: Directory traversal; emit OCEL traces to disk; filter ignored directories
- Virtual documents: Verify `anti-llm://lsp318-matrix` renders LSP 3.18 capability matrix with BLAKE3 receipts
- Dogfood tests (`cargo test -p anti-llm-cheat-lsp`): 30+ tests validate canary rules and gate logic

**Expected Output:**
```
LSP: Initialize response with virtual document URIs; diagnostics keyed by rule (e.g., ANTI-LLM-PLAIN-TOWER-LSP)
Scan: "Observations: N, Diagnostics emitted: M" with per-file, per-line findings
Tests: All 30 dogfood tests ADMITTED (no rule violations, receipts present)
```

**README:** [Present](#anti-llm-cheat-lspreadmemd) at `examples/anti-llm-cheat-lsp/README.md`

---

## 3. clap-noun-verb-lsp

**Description:** CLI noun/verb grammar integration example. Turns a `clap-noun-verb` command structure into an inspectable, diagnosable language surface with code actions and dynamic capability registration.

**Key Files:**
- `src/server.rs` — Core `CommandLspBackend`; dynamic registration, virtual documents for command graph
- `src/command.rs` — Command graph extraction and validation
- `src/actions.rs` — Code action generation (e.g., command hints, fix suggestions)
- `src/hierarchy.rs` — Noun/verb structure analysis
- `src/edits.rs` — Text edit application for code actions

**Run:**
```bash
cargo run -p clap-noun-verb-lsp
```

**What to Test:**
- Open a CLI definition file; LSP extracts noun/verb grammar
- Emit diagnostics for missing verbs, invalid nouns, or ambiguous commands
- Code actions suggest rewrites of malformed commands
- Dynamic registration: Capabilities register/unregister as workspace state changes
- Pull virtual documents (e.g., `clap-noun-verb://command-graph`) to inspect parsed structure
- Semantic tokens, inlay hints, inline values for command metadata

**Expected Output:**
```
LSP: Capabilities include codeActionProvider, diagnosticProvider, dynamicRegistration
Virtual documents render command graph as text or JSON
Code actions emitted with fixes for invalid command syntax
Receipts for major LSP requests (pull, hover, codeAction)
```

**README:** [Present](#clap-noun-verb-lspreadmemd) at `examples/clap-noun-verb-lsp/README.md` (contains feature summary and test instructions)

---

## 4. axum-lsp

**Description:** Alternative transport over HTTP using tower and axum integration. Demonstrates HTTP-based LSP instead of stdio; routes LSP methods through tower middleware and applies domain-specific rules for async code patterns.

**Key Files:**
- `src/main.rs` — `AxumBackend` implementation; HTTP routing, towerwares; inline Axum-specific lint rules
- `src/semantics.rs` — Semantic analysis stub for Axum patterns

**Run:**
```bash
cargo run -p axum-lsp -- start --stdio
```
(Note: Despite `--stdio`, this example demonstrates HTTP-ready transport plumbing; see source for towerwares integration.)

**What to Test:**
- Open a Rust file; LSP scans for Axum anti-patterns:
  - `unwrap()` in handlers → AXUM-001 ERROR
  - Blocking calls (`std::thread::sleep`, `std::fs::read`) in async context → AXUM-002 WARNING
- Diagnostics published with rule ID and remediation message
- Full text document sync on open/change/close

**Expected Output:**
```
LSP: ServerCapabilities with textDocumentSync (FULL)
Diagnostics for each matched Axum rule, with source="axum-lsp"
Code examples of blocking/unwrap patterns flagged
```

**README:** [Present](#axum-lspreadmemd) at `examples/axum-lsp/README.md` (generic; consider expanding with Axum-specific patterns)

---

## 5. bevy-lsp

**Description:** Game engine LSP over Bevy. Demonstrates async resource integration and tree-sitter AST codegen for domain-specific syntax. Integrates `AutoLspAdapter` for automatic AST observation.

**Key Files:**
- `src/main.rs` — Server entry point; `LspService` wrapping `BevyBackend`
- `src/backend.rs` — `BevyBackend` implementation; Bevy entity/component analysis, async resource state
- `src/semantics.rs` — Semantic token rules for Bevy systems, queries, components
- `build.rs` — Tree-sitter codegen (`lsp-max-ast-codegen`) for Rust AST

**Run:**
```bash
cargo run -p bevy-lsp
```

**What to Test:**
- Open a Bevy system definition (Rust file with `#[system]` macros, `Query`, `Res`); LSP parses via tree-sitter
- Emit diagnostics for invalid component queries, resource access patterns
- Semantic tokens highlight entity references, system boundaries
- Inlay hints show system execution order or archetype hints
- Test with sample Bevy code (ECS patterns)

**Expected Output:**
```
LSP: Initialize response with semantic token types for Bevy concepts
Diagnostics for invalid queries/resources with fix suggestions
Semantic tokens colored by entity/component/system boundaries
AutoLspAdapter integration confirms AST observation is live
```

**README:** [Present](#bevy-lspreadmemd) at `examples/bevy-lsp/README.md` (generic; consider expanding with ECS pattern examples)

---

## 6. tex-lsp

**Description:** Real-world LaTeX language server with document store and incremental parsing. Uses tree-sitter-latex for AST observation; implements code completion, structure navigation, and multi-file cross-references.

**Key Files:**
- `src/main.rs` — Entry point; `#[verb("start", "server")]` CLI; runs `pdflatex` / `bibtex` for build integration
- `src/backend.rs` — `TexBackend`; document store, incremental re-parse, reference tracking
- `src/semantics.rs` — Semantic token rules for LaTeX constructs (sections, labels, citations)
- `build.rs` — Tree-sitter codegen for LaTeX grammar
- Thesis directory integration (`docs/thesis/periodic-table-of-reason/`) with PDF build artifacts

**Run:**
```bash
cargo run -p tex-lsp -- start server
```

**What to Test:**
- Open a `.tex` file from the thesis directory; LSP parses via tree-sitter-latex
- Publish diagnostics for mismatched environments, undefined labels, citation errors
- Code completion for `\cite{}`, `\ref{}`, cross-file `\input{}`
- Document symbols (sections, subsections, labels) for outline navigation
- Hover over references to show target definitions
- Incremental parsing on edit; update affected dependent files
- Build integration: `pdflatex` and `bibtex` invoke on save (if configured)

**Expected Output:**
```
LSP: ServerCapabilities include documentSymbolProvider, completionProvider, hoverProvider
Diagnostics for LaTeX errors (mismatched {}, undefined refs)
Semantic tokens for \commands, {arguments}, [options]
Build status on save (pdflatex exit code, PDF size)
```

**README:** [Present](#tex-lspreadmemd) at `examples/tex-lsp/README.md` (generic; consider expanding with thesis-specific workflow docs)

---

## 7. wasm4pm-lsp

**Description:** Process mining LSP over the wasm4pm execution engine. Demonstrates gate conformance validation and receipt-based admission. Includes 4 dogfood tests (`dogfood_gc00*.rs`) validating LSP 3.18 features and gate conformance.

**Key Files:**
- `src/main.rs` — Monolithic server; `wasm4pm` engine integration, process model execution, capability vectors, virtual documents
- `../gc005-wasm4pm-adapter/src/` — Adapter layer between lsp-max and wasm4pm types
- `tests/dogfood_gc004.rs` — Gate conformance (law-axis admission)
- `tests/dogfood_gc005.rs` — Receipt validation
- `tests/dogfood_gc006_calver.rs` — CalVer version law enforcement
- `tests/dogfood_gc007_lsp318.rs` — LSP 3.18 feature completeness with transcripts

**Run (LSP mode):**
```bash
cargo run -p wasm4pm-lsp
```

**Run (Tests):**
```bash
cargo test -p wasm4pm-lsp
```

**What to Test:**
- Initialize LSP; verify `ConformanceVector` with `admitted`/`refused`/`unknown` axes
- Publish diagnostic bundles for process model conformance (control flow, resource usage)
- Virtual documents:
  - `wasm4pm://models` — list of loaded process models
  - `wasm4pm://conformance` — conformance matrix with receipts
  - `wasm4pm://gates` — gate status and ANDON signals
- Dogfood tests validate:
  - Gate conformance (all laws admitted or blocked, never implicit)
  - Receipt artifacts (BLAKE3 signatures, checkpoint paths)
  - CalVer version compliance (26.M.D format)
  - LSP 3.18 feature matrix with explicit negative-control results
- Run the 4 dogfood tests; all must exit 0 with ADMITTED or BLOCKED (no UNKNOWN in final verdict)

**Expected Output:**
```
LSP: Initialize response with proposed LSP 3.18 capabilities
Diagnostics bundles (OCEL traces) for process model execution
Virtual documents with conformance vectors and gate status
Dogfood tests: ✓ gc004 (gate ADMITTED), ✓ gc005 (receipts validated), ✓ gc006 (CalVer ADMITTED), ✓ gc007 (LSP 3.18 transcript ADMITTED)
ANDON gate clears when all diagnostics resolved
```

**README:** Not present; [create `wasm4pm-lsp/README.md`](#creating-individual-readme-files).

---

## Running All Examples

Verify all examples compile and tests pass:

```bash
# Compile all examples
cargo build --workspace

# Test all examples
cargo test --workspace

# Lint and format
just dx-polish

# Architectural boundary verification (checks lsp-types-max and sibling repos)
just dx-verify
```

## Example Dependencies

All examples depend on:
- **lsp-max** (`../../`) — the LSP framework
- **lsp-types-max** (`../../../lsp-types-max`) — LSP 3.18 type definitions (path dependency, version 26.6.8+)
- **clap-noun-verb** (version 26.6.2+) — CLI grammar framework (most examples use this)

Some examples also depend on:
- **tree-sitter**, **tree-sitter-rust**, **tree-sitter-latex** — AST parsing
- **wasm4pm** — process mining engine (wasm4pm-lsp only)
- **lsp-max-ast-codegen** — automatic AST adapter generation (bevy-lsp, tex-lsp)

## Creating Individual README Files

The following examples currently lack README.md documentation:

### pattern-lsp/README.md

Template:
```markdown
# pattern-lsp

Regex-based pattern detection language server using lsp-max.

## Features

- Rule-based diagnostics for custom text patterns
- Real-time analysis on document open/change/save
- Configurable severity levels (ERROR, WARNING, INFO, HINT)

## Running

\`\`\`bash
cargo run -p pattern-lsp -- serve --stdio
\`\`\`

## Testing

\`\`\`bash
cargo test -p pattern-lsp
\`\`\`

## Rules

Define patterns in `src/rules.rs`. Each rule maps a regex to a diagnostic.

\`\`\`
RULE-001: matches /pattern/
RULE-002: matches /another-pattern/
\`\`\`

Expected output: Diagnostics published for each match.
```

### wasm4pm-lsp/README.md

Template:
```markdown
# wasm4pm-lsp

Process mining language server for wasm4pm workspaces.

## Features

- Gate conformance validation via gc005-wasm4pm-adapter
- BLAKE3 receipt-based admission tracking
- CalVer version law enforcement (26.M.D)
- LSP 3.18 feature matrix with negative-control tests

## Running

\`\`\`bash
cargo run -p wasm4pm-lsp
\`\`\`

## Testing

\`\`\`bash
cargo test -p wasm4pm-lsp
\`\`\`

Dogfood tests validate:
- gc004: Gate conformance (ADMITTED or BLOCKED)
- gc005: Receipt artifacts with BLAKE3 signatures
- gc006: CalVer version compliance
- gc007: LSP 3.18 feature completeness

Expected output: All tests ADMITTED; no UNKNOWN verdicts.
```

## Notes

- **Publication:** All examples have `publish = false` in Cargo.toml (workspace-only, not published to crates.io)
- **Law Enforcement:** The anti-llm-cheat-lsp example detects violations of the laws documented in `AGENTS.md` and enforces them via the ANDON gate. All other examples must not introduce the forbidden patterns it detects
- **Receipts:** LSP 3.18 features claimed in dogfood tests must include BLAKE3-signed receipt artifacts (path, digest, checkpoint). Log output alone is not a receipt
- **Virtual Documents:** Read-only URIs like `anti-llm://...`, `wasm4pm://...` expose internal state without mutating files directly — the LSP surface is read-only
