# Contributing to lsp-max

Read `AGENTS.md` and any nested `AGENTS.md` files that govern the subtree you change. Repository doctrine, executable checks, and exact observed results outrank prose claims.

## Development environment

Requirements:

- Git
- The pinned `nightly-2026-04-15` toolchain from `rust-toolchain.toml`
- `rustfmt` and Clippy for that pinned toolchain
- `ripgrep`
- Docker for reproducing the clean-container build

The manifests declare Rust `1.87.0` as the dependency-resolution floor. This is not a stable-Rust support claim: `wasm4pm-compat` requires nightly features, so the pinned nightly is the only admitted compiler identity for workspace execution.

```bash
git clone <your-fork>
cd lsp-max
rustup show
rustc +nightly-2026-04-15 --version --verbose
cargo +nightly-2026-04-15 metadata --format-version 1 --no-deps
```

The repository must resolve from repository-local paths and published crates. Do not require adjacent sibling checkouts, absolute machine paths, fabricated dependency stubs, or generated source committed only to satisfy CI.

## Change discipline

1. Start from an exact base SHA and use a purpose-named branch.
2. Preserve public interfaces and generated/source boundaries unless the task requires changing them.
3. Make the smallest coherent diff that closes the observed failure.
4. Add a positive control and a negative control for behavior or admission changes.
5. Do not weaken, skip, or mask a failing check. `continue-on-error: true`, `|| true`, and piped commands that discard the producer exit status are prohibited in authoritative gates.
6. Do not claim execution from inspection. Report observed, inferred, blocked, and unsupported states separately.

## Required verification

Run the cheapest relevant check first, then expand after it succeeds.

```bash
bash scripts/audit-repository.sh
cargo +nightly-2026-04-15 fmt --all -- --check
cargo +nightly-2026-04-15 check -p lsp-max --lib
cargo +nightly-2026-04-15 check -p lsp-max --lib --no-default-features --features runtime-agnostic
cargo +nightly-2026-04-15 clippy --workspace --all-targets --all-features -- -D warnings
cargo +nightly-2026-04-15 test --workspace --all-targets
cargo +nightly-2026-04-15 publish -p lsp-max --dry-run --allow-dirty
```

For the clean execution boundary:

```bash
docker build --progress=plain --file Dockerfile.ci --tag lsp-max-rust:local .
```

Use the exact command requested by an issue or acceptance criterion when it is stricter than this baseline. A narrow unit test is not a substitute for a requested CLI, integration, LSP transport, or end-to-end proof.

## Rust standards

- Prefer typed errors and explicit refusal variants over generic strings or panics.
- Avoid `unwrap`, `expect`, `panic!`, `todo!`, and `unimplemented!` outside tests unless an invariant makes the state unconstructable and the justification is documented.
- Keep stdout reserved for LSP protocol frames. Route diagnostics through `tracing` or stderr.
- Preserve `admitted`, `refused`, and `unknown` as disjoint states.
- Keep hot-path syntax ownership, durable structure, semantic storage, history, and receipts in their documented layers.
- Avoid new dependencies when the standard library or an existing workspace dependency is sufficient.

## Tests and receipts

Tests demonstrate observed behavior; they are not themselves cryptographic receipts. Receipt-bearing changes must test:

- subject identity and authority
- signature or digest verification
- linkage and sequence continuity
- tamper refusal
- replay or duplicate handling
- serialization round trips when persisted or transported

Slow or environment-dependent tests may be ignored only with an explicit reason and a separate CI path that executes them.

## Pull requests

Keep pull requests draft until the exact published head has completed the required checks. The description must state:

- base and head SHAs
- observed failure or gap
- changed boundaries
- commands executed and exit results
- tests added, including negative controls
- generated-file status
- known unverified boundaries and falsifiers

Do not merge unless explicitly authorized. Do not use real `cargo publish` in CI or agent execution; only `cargo publish --dry-run` is admitted for automated validation.

## Commit messages

Use a conventional type and a concrete bounded subject, for example:

```text
fix(receipts): refuse broken sequence linkage
ci(quality): execute exact-head workspace gates
```

Avoid unbounded victory claims. Prefer `ALIVE`, `PARTIAL_ALIVE`, `BLOCKED`, `BUILD_BROKEN`, `UNSUPPORTED`, or a typed `REFUSED` status with its tested boundary.
