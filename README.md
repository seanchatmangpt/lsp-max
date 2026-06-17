# lsp-max

[![Build Status][build-badge]][build-url]
[![Crates.io][crates-badge]][crates-url]
[![License][license-badge]][license-url]
[![Rust 1.70+][rust-badge]][rust-url]

[build-badge]: https://github.com/seanchatmangpt/lsp-max/workflows/rust/badge.svg
[build-url]: https://github.com/seanchatmangpt/lsp-max/actions
[crates-badge]: https://img.shields.io/crates/v/lsp-max.svg?label=26.6.9
[crates-url]: https://crates.io/crates/lsp-max
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue
[license-url]: #license
[rust-badge]: https://img.shields.io/badge/rust-1.70%2B-orange
[rust-url]: https://www.rust-lang.org

Law-state LSP runtime that projects a multidimensional state machine through LSP 3.18. Provides maximum protocol coverage, process-mining conformance, and receipt-chain admission for agents, CI systems, and release gates—not tower-lsp, but a distinct runtime with extended `max/*` surfaces for introspection, gates, snapshots, and law enforcement.

## Quick start

**Clone and build** (≤5s on recent hardware):
```bash
git clone https://github.com/seanchatmangpt/lsp-max.git
cd lsp-max
cargo test --workspace
```

**Run an example**:
```bash
cargo run --example anti-llm-cheat-lsp
```

**Use in your project**:
```toml
[dependencies]
lsp-max = "26.6"
```

## Key features

| Feature | Status | Notes |
|---------|--------|-------|
| LSP 3.18 capability coverage | ~87% | See `docs/FEATURES.md` |
| ConformanceVector (`admitted`/`refused`/`unknown`) | Supported | Axes never collapse; unknown is preserved |
| Process-mining via wasm4pm | Supported | OCEL event logs from OTel traces, checked against declared models |
| Receipt-chain admission | Supported | BLAKE3-hashed receipts required; tests without receipts rejected |
| Λ_CD gate (PreToolUse enforcement) | Supported | CI gate blocks shell actions while `WASM4PM-*`/`GGEN-*` diagnostics active |
| Anti-LLM diagnostics | Supported | Detects tower-lsp references, victory language, fake receipts, contract violations |
| CalVer versioning | Enforced | `26.6.9` = 2026-06-09; version mismatches are diagnostic events |
| Multi-server compositor | Supported | Fans to child servers, merges diagnostics with quorum debounce, emits receipts |

## Directory structure

```
lsp-max/
├── src/                    # LSP server framework core
│   ├── lib.rs             # Main crate interface
│   ├── language_server.rs # LanguageServer trait definition
│   ├── service.rs         # LspService orchestration
│   ├── gate.rs            # Λ_CD gate state machine
│   ├── diagnostics.rs     # Law-state diagnostic engine
│   ├── composition/       # Multi-server compositor internals
│   └── primitives/        # Law-state value types (Receipt, ConformanceVector, etc.)
│
├── lsp-max-protocol/      # max/* method declarations, capability vectors, receipts
├── lsp-max-macros/        # Proc macros (#[lsp_max::async_trait], etc.)
├── lsp-max-runtime/       # Typestate machine, phases, transitions, snapshots
├── lsp-max-agent/         # Agent integration, analysis bundles
│
├── crates/
│   ├── lsp-max-cli/       # Noun/verb CLI (clap-noun-verb-based actuation grammar)
│   ├── lsp-max-client/    # LSP client framework (drives servers in tests)
│   ├── lsp-max-compositor/ # Multi-server hub with gate, quorum debounce, receipts
│   ├── lsp-max-base/      # Base LSP type extensions
│   ├── lsp-max-live/      # Live protocol surfaces (streaming diagnostics)
│   ├── lsp-max-lsif/      # LSIF export and conformance checking
│   ├── lsp-max-specgen/   # Codegen from LSP 3.18 metaModel.json
│   ├── lsp-max-adapters/  # Tree-sitter-driven AST/codegen (ported auto-lsp stack)
│   ├── playground/        # Dev-dependency test harness with demo binaries
│   └── lsif-*/            # LSIF indexing and linking tools
│
├── examples/              # Domain-specific dogfood servers
│   ├── anti-llm-cheat-lsp/  # Diagnostic canary (detects tower-lsp, fake receipts, victory language)
│   ├── clap-noun-verb-lsp/  # Noun/verb CLI demo
│   ├── pattern-lsp/         # Pattern detection
│   ├── wasm4pm-lsp/         # Process-mining over wasm4pm
│   ├── axum-lsp/            # Axum framework integration
│   ├── bevy-lsp/            # Bevy framework integration
│   ├── tex-lsp/             # TeX/LaTeX LSP
│   └── *.rs                 # Explanation crates (receipt_chain_explained, conformance_vector_explained, etc.)
│
├── tests/                 # Integration and e2e tests
│   ├── e2e/              # End-to-end test suites
│   ├── lsp318_capabilities/ # LSP 3.18 feature validation
│   ├── dogfood_loop/     # Dogfood test drivers
│   └── common/           # Shared test utilities
│
├── web/                  # Next.js dashboard and analytics
│   ├── app/             # Next.js App Router
│   ├── lib/             # Shared components and utilities
│   └── REPRESENTATION_MAP.md  # Dashboard data model
│
├── docs/                # Narrative and reference documentation
│   ├── FEATURES.md              # LSP 3.18 coverage matrix with receipts
│   ├── EXAMPLES.md              # Diataxis-mapped example index
│   ├── TEST_INFRA.md            # Test architecture and conformance
│   ├── CANCELLATION_SAFETY.md   # Async cancellation guarantees
│   ├── ADR/                     # Architectural Decision Records
│   ├── law/                     # Law-state semantics papers
│   ├── papers/                  # Academic references
│   ├── reports/                 # Analysis and audit reports
│   └── thesis/                  # Theoretical foundation documents
│
├── CLAUDE.md            # Codebase laws and instructions (enforced by anti-llm-cheat-lsp)
├── AGENTS.md            # Agent protocol, conformance laws, admission rules
├── CONTRIBUTING.md      # Development guidelines
├── Justfile             # Build orchestration (just test, just dx-verify, etc.)
└── README.md            # This file
```

## Published crates

| Crate | Description |
|-------|-------------|
| `lsp-max` | LSP server framework: `LanguageServer` trait, `LspService`, `Server` over stdio/TCP, law-state surface |
| `lsp-max-macros` | Proc macros for async traits and attribute derivation |
| `lsp-max-protocol` | `max/*` method definitions, `MaxDiagnostic`, `ConformanceVector`, receipt types |
| `lsp-max-cli` | Noun/verb CLI for gate checks, server control, and diagnostics |
| `lsp-max-client` | LSP client for test harnesses and agent-driven server control |
| `lsp-max-compositor` | Multi-server fan-out hub with Λ_CD gate and receipt emission |

All other workspace crates have `publish = false`.

## Design principles

- **No victory language**: Status values are bounded (`ADMITTED`, `CANDIDATE`, `BLOCKED`, `REFUSED`, `UNKNOWN`, `PARTIAL`, `OPEN`) — never "done," "solved," or "guaranteed."
- **Unknown is preserved**: `ConformanceVector` axes never collapse unknown into admitted or refused; ambiguity is explicit.
- **Receipts, not logs**: Capability claims require BLAKE3-hashed receipt artifacts with path, digest, boundary, and checkpoint — test stdout is not a receipt.
- **Read-only LSP surface**: The server emits diagnostics, hovers, and intents but never mutates files; all mutation is client-driven.
- **CalVer, not SemVer**: Version `26.6.9` encodes the date (2026-06-09); mismatches are diagnostic events.
- **Distinct from tower-lsp**: Never reference plain `tower_lsp` or `tower-lsp` in code, manifests, or docs (outside negative-control fixtures).

## Build & test

Run `just` alone to list recipes:

```bash
just test                 # cargo test --workspace
just test-e2e             # cargo test --test e2e
just test-pre-publish     # dx-verify + dx-polish + all tests (≤10s)
just dx-polish            # cargo fmt --all + clippy -D warnings
just dx-verify            # Architectural boundary scan across sibling repos
```

Single test or crate:

```bash
cargo test -p lsp-max
cargo test --test test_lsp318_capabilities
cargo test -p anti-llm-cheat-lsp --test dogfood
```

## Sibling dependencies

The workspace requires sibling checkouts at:

- `../lsp-types-max` — LSP 3.18 type authority (with `proposed` features)
- `../wasm4pm-compat` — Process-mining type baseline
- `../wasm4pm` — Execution engine

No intermediary type crates (`wasm4pm_types`, `ocel_core`, etc.) are allowed.

## Documentation

- **[GETTING_STARTED.md](GETTING_STARTED.md)** — Setup and first server
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — Development workflow and law enforcement
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** — Five-layer model, typestate machine, Λ_CD gate
- **[docs/FEATURES.md](docs/FEATURES.md)** — LSP 3.18 coverage with receipts
- **[DOC_COVERAGE_LOG.md](DOC_COVERAGE_LOG.md)** — Documentation audit and gaps
- **[CLAUDE.md](CLAUDE.md)** — Codebase laws and constraints (consulted by anti-llm-cheat-lsp)
- **[AGENTS.md](AGENTS.md)** — Agent protocol and conformance rules

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) at your option.

---

**Rust version**: 1.70+ | **CalVer**: 26.6.9 (2026-06-09) | **Status**: CANDIDATE
