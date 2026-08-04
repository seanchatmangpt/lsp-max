# lsp-max

[![Rust Quality][build-badge]][build-url]
[![License][license-badge]][license-url]
[![Rust 1.87+][rust-badge]][rust-url]

[build-badge]: https://github.com/seanchatmangpt/lsp-max/actions/workflows/rust-quality.yml/badge.svg
[build-url]: https://github.com/seanchatmangpt/lsp-max/actions/workflows/rust-quality.yml
[license-badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue
[license-url]: #license
[rust-badge]: https://img.shields.io/badge/rust-1.87%2B-orange
[rust-url]: https://www.rust-lang.org

A post-human LSP 3.18 runtime for autonomous agents. `lsp-max` enforces architectural laws via cryptographic receipt chains, three-valued conformance vectors, and deterministic gates. It is not an IDE helper; it is an admission controller for machine agent workflows.

## Quick start

```bash
git clone https://github.com/seanchatmangpt/lsp-max.git
cd lsp-max
rustup show
cargo test --workspace --all-targets
```

The workspace resolves published external crates and repository-local path dependencies only. No adjacent sibling checkout or bootstrap script is required.

## Using lsp-max

**As a library:**

```toml
[dependencies]
lsp-max = "26.7"
```

**Run an included server example:**

```bash
cargo run -p powl-lsp -- start server
```

Extend with the `RulePackServer` trait and a TOML rule file instead of reproducing LSP transport and routing boilerplate. See `examples/` for reference implementations.

## What is this?

`lsp-max` is a law-state runtime projected through LSP. It enforces invariants, maintains cryptographic receipts, and gates state transitions through formal predicates. Every LSP call is a state-transition attempt. Valid transitions produce receipts; invalid transitions emit ANDON refusal diagnostics.

**Core features:**

- **Law enforcement:** Receipt chains bind state transitions to evidence.
- **Conformance vectors:** Three-axis tracking separates admitted, refused, and unknown states.
- **Multi-server composition:** Fan-out diagnostics and tier-stratified routing.
- **Process mining:** DFG fitness and Declare constraint validation over LSP event logs.
- **Specification-driven protocol:** Generate protocol types from the LSP meta-model.
- **Agent integration:** Lifecycle hooks for discovery and analysis.
- **Automated gates:** Exact-head CI validates repository invariants, MSRV, formatting, linting, tests, packaging, and clean Docker builds.

## Verification

```bash
bash scripts/audit-repository.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo publish -p lsp-max --dry-run --allow-dirty
```

`cargo publish` without `--dry-run` is outside automated execution authority.

## Documentation

**Start here:** [`docs/README.md`](docs/README.md)

- **[Tutorials](docs/tutorials/)** — Build a complete agent loop
- **[How-to Guides](docs/how-to/)** — Task-oriented operational recipes
- **[Reference](docs/reference/)** — Protocol, configuration, and testing references
- **[Explanation](docs/explanation/)** — Architectural rationale

Additional governance and release material:

- [`docs/rfcs/README.md`](docs/rfcs/README.md)
- [`docs/book/01-architecture.md`](docs/book/01-architecture.md)
- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`SECURITY.md`](SECURITY.md)
- [`docs/how-to/release.md`](docs/how-to/release.md)
- [`DEFINITION_OF_DONE.md`](DEFINITION_OF_DONE.md)
- [`CHANGELOG.md`](CHANGELOG.md)

## License

Licensed under either the Apache License, Version 2.0, or the MIT license at your option.
