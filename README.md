# lsp-max

[![Build Status][build-badge]][build-url]
[![License][license-badge]][license-url]
[![Rust 1.82+][rust-badge]][rust-url]

[build-badge]: https://github.com/seanchatmangpt/lsp-max/workflows/rust/badge.svg
[build-url]: https://github.com/seanchatmangpt/lsp-max/actions
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue
[license-url]: #license
[rust-badge]: https://img.shields.io/badge/rust-1.82%2B-orange
[rust-url]: https://www.rust-lang.org

A post-human LSP 3.18 runtime for autonomous agents. `lsp-max` enforces architectural laws via cryptographic receipt chains, three-valued conformance vectors, and deterministic gates. It is not an IDE helper; it is an admission controller for machine agent workflows.

## Quick start

```bash
git clone https://github.com/seanchatmangpt/lsp-max.git
cd lsp-max
bash scripts/bootstrap.sh  # fetch sibling repos
cargo test --workspace
```

> **Workspace setup:** This repo depends on three siblings (`../lsp-types-max`, `../wasm4pm-compat`, `../wasm4pm`). Run `bash scripts/bootstrap.sh` to fetch them. In Claude Code, `SessionStart` hooks bootstrap automatically.

## Using lsp-max

**As a library:**

```toml
[dependencies]
lsp-max = "26.7"  # CalVer: YY.M.D scheme
```

**As a server:**

```bash
cargo run --bin lsp-max -- --config lsp-max.toml
```

Extend with the `RulePackServer` trait (20 LOC) + a TOML rule file (50 LOC) instead of writing 400+ LOC of LSP boilerplate. See `examples/` for reference implementations.

## What is this?

`lsp-max` is a law-state runtime projected through LSP — it enforces invariants, maintains cryptographic receipts, and gates state transitions via formal predicates. Every LSP call is a state-transition attempt. Valid transitions produce receipts; invalid transitions emit ANDON (refusal) diagnostics.

**Core features:**

- **Law enforcement:** Receipt chains prove every state transition; no mutation without cryptographic proof.
- **Conformance vectors:** Three-axis tracking (admitted/refused/unknown) instead of binary support flags.
- **Multi-server composition:** Fan-out diagnostics to multiple servers; tier-stratified routing (Primary/Secondary/DiagnosticsOnly).
- **Process mining:** DFG fitness and Declare constraint validation over LSP event logs.
- **Specification-driven:** Generate protocol types from LSP `metaModel.json`; extend via `RulePackServer` trait.
- **Agent integration:** Hook lifecycle (SessionStart, PreToolUse, PostToolUse, SubagentStart/Stop) for agent discovery and analysis.
- **Automated gates:** Conformance score calculated as `max(0, 100 - ∑penalties)`. Release admitted only if score = 100.0.

## ggen-first manufacturing

The canonical manufacturing authority is [`packs/lsp-max-runtime-pack/ontology.ttl`](packs/lsp-max-runtime-pack/ontology.ttl). [`ggen.toml`](ggen.toml) projects that admitted graph into governed documentation, evidence manifests, a Rust law contract, and the exact-head GitHub Actions verifier. Generated files carry `@generated` provenance and must be changed through the graph or template, followed by sync, verification, receipt, and replay.

The graph preserves the repository's storage law:

- Salsa owns hot incremental computation, not Tree-sitter trees.
- Papaya stages hot diagnostics.
- LSIF stores durable, receipt-bound structure.
- Oxigraph stores cold meaning outside the `didChange` hot path.
- OCEL stores process history.
- LSP projects live law.
- BLAKE3 receipts establish bounded standing.

It also imports the current ggen Building Block and standing discipline: lifecycle is orthogonal to standing; BRCE is the only lawful actuation path; `ALIVE` requires a witness, falsifier, independent verifier, receipt verifier, and replay over one admitted input closure.

```bash
# Manufacture and refuse tracked drift
just ggen-drift

# Admit the semantic graph and execute typed falsifiers
just ggen-semantic

# Emit and replay a BLAKE3-bound execution receipt
just ggen-receipt

# Execute the generated Rust witness
just ggen-rust-contract
```

The program contract begins at `UNKNOWN`. The independent semantic verifier may establish `PARTIAL_ALIVE` for its bounded graph/fixture surface. It does not establish crown standing or claim the full LSP runtime is `ALIVE`.

Generated references:

- [`docs/generated/GGEN_MANUFACTURING_CONTRACT.md`](docs/generated/GGEN_MANUFACTURING_CONTRACT.md)
- [`docs/generated/STANDING_MODEL.md`](docs/generated/STANDING_MODEL.md)
- [`docs/generated/GALL_ROADMAP.md`](docs/generated/GALL_ROADMAP.md)
- [`evidence/generated/verification-manifest.json`](evidence/generated/verification-manifest.json)

## Documentation

**Start here:** [`docs/README.md`](docs/README.md) — Documentation overview using the Diataxis framework.

- **[Tutorials](docs/tutorials/)** — Learn lsp-max by building a complete agent loop
- **[How-to Guides](docs/how-to/)** — Task-oriented recipes (blocking on ANDON, releasing, etc.)
- **[Reference](docs/reference/)** — Complete protocol spec, configuration, testing patterns
- **[Explanation](docs/explanation/)** — Understand why LSP is the law-state substrate

**Design decisions:** See [`docs/rfcs/README.md`](docs/rfcs/README.md) for the Accepted RFCs that govern the architecture.

**Architecture overview:** [`docs/book/01-architecture.md`](docs/book/01-architecture.md) — Comprehensive system overview.

**Contributing:** [`CONTRIBUTING.md`](CONTRIBUTING.md) — Coding standards, git workflow, and development setup.

**Release process:** [`docs/how-to/release.md`](docs/how-to/release.md) — Version bumping, pre-release checklist, dry-run publish, and manual publish instructions.

**Definition of Done:** [`DEFINITION_OF_DONE.md`](DEFINITION_OF_DONE.md) — Release admission gates for the current version.

**Changelog:** [`CHANGELOG.md`](CHANGELOG.md) — Version history and release notes (CalVer: YY.M.D).

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
