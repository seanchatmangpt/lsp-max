# Getting Started with lsp-max

Welcome to **lsp-max**: a law-state LSP runtime that maximizes LSP 3.18 coverage with process-mining conformance and receipt-chain admission.

Status: **CANDIDATE** — Build and tests functional when sibling repos are present.

---

## Prerequisites

You'll need:

- **Rust 1.82.0+** (check via `rustc --version`)
  - Install from [rustup.rs](https://rustup.rs) if needed
  - Upgrade: `rustup update`
  
- **`just`** — Task runner (like `make` but better)
  - Install: `cargo install just`
  - Or: see [casey/just](https://github.com/casey/just#installation)

- **`cargo-deny`** — Dependency auditing
  - Install: `cargo install cargo-deny`

- **`cargo-edit`** — Cargo subcommands (add/remove deps)
  - Install: `cargo install cargo-edit`

- **`rg` (ripgrep)** — Fast grep, used by `just dx-verify`
  - Install: `cargo install ripgrep` or package manager

- **Git** — For cloning and checking sibling repos

---

## Clone and Bootstrap

### 1. Clone lsp-max

```bash
git clone https://github.com/seanchatmangpt/lsp-max.git
cd lsp-max
```

### 2. Clone and verify sibling repos (REQUIRED for build)

lsp-max does **not** build standalone. You must checkout the following sibling repositories at the parent level:

```bash
cd ..

# Clone the LSP type authority (proposed LSP 3.18 features)
git clone https://github.com/seanchatmangpt/lsp-types-max.git

# Clone the process-mining baseline type authority (wasm4pm-compat)
git clone https://github.com/seanchatmangpt/wasm4pm-compat.git

# Clone the execution engine
git clone https://github.com/seanchatmangpt/wasm4pm.git

# Return to lsp-max
cd lsp-max
```

Expected directory layout:
```
~/
├── lsp-max/                    (you are here)
├── lsp-types-max/             (path dependency)
├── wasm4pm-compat/            ([patch.crates-io] baseline type authority)
└── wasm4pm/                   ([patch.crates-io] execution engine)
```

### 3. Build the workspace

```bash
cargo build --workspace
```

If you see path-dependency errors, verify the sibling repos are at the correct locations (see above).

---

## Quick Test

Run the test suite to verify your setup:

```bash
just test
```

**Expected:**
- All workspace tests pass
- Wall-clock time ≤ 5 seconds for smoke tests
- Full suite typically 30–60 seconds on modern hardware

To run a single test:
```bash
cargo test -p lsp-max --lib               # One crate
cargo test --test test_lsp318_capabilities # One integration test
cargo test -p pattern-lsp                 # Example crate
```

Run with verbose output:
```bash
cargo test -- --nocapture
```

---

## Workspace Architecture

lsp-max follows a **five-layer model**:

```
┌─────────────────────────────────────────────────┐
│ (5) Autonomic LSP Mesh                          │
│     (compositor, multi-server federation)       │
└─────────────────────────────────────────────────┘
                      ↑
┌─────────────────────────────────────────────────┐
│ (4) Knowledge Hooks                             │
│     (tracing, telemetry, admission receipts)    │
└─────────────────────────────────────────────────┘
                      ↑
┌─────────────────────────────────────────────────┐
│ (3) Law-State Runtime                           │
│     (typestate machine, phases, snapshots)      │
└─────────────────────────────────────────────────┘
                      ↑
┌─────────────────────────────────────────────────┐
│ (2) Local LSP State Surface                     │
│     (diagnostics, hovers, conformance vectors)  │
└─────────────────────────────────────────────────┘
                      ↑
┌─────────────────────────────────────────────────┐
│ (1) Actuation Grammar                           │
│     (clap-noun-verb CLI, file-based routing)    │
└─────────────────────────────────────────────────┘
```

### Core Crates

**Root crate** (`src/`)
- LSP server framework: `LanguageServer` trait, `LspService`, `Server` over stdio/TCP
- Law-state surface: `gate.rs`, `diagnostics.rs`, `composition.rs`

**Protocol** (`lsp-max-protocol/`)
- `max/*` method declarations (snapshots, conformance vectors, receipts)
- `MaxDiagnostic`, `ConformanceVector`, analysis bundles
- **Where to change protocol types**

**Runtime** (`lsp-max-runtime/`)
- Typestate machine: phases and transitions
- Runtime snapshotting
- `ANDON` gate state machine

**Agent Integration** (`lsp-max-agent/`)
- Agent-facing analysis bundles
- Receipt chain validation

**Macros** (`lsp-max-macros/`)
- `#[lsp_max::async_trait]` and internal codegen macros

### Workspace Crates

| Layer | Crate | Purpose |
|-------|-------|---------|
| 1     | `lsp-max-cli` | Actuation: noun/verb CLI on `clap-noun-verb` |
| 2     | `lsp-max-base`, `-live`, `-lsif`, `-specgen` | Protocol types, LSIF export, LSP 3.18 codegen |
| 2     | `lsp-max-client` | LSP client framework (for tests) |
| 3     | `lsp-max-compositor` | Multi-server fan-out, Λ_CD gate, receipt emission |
| 4     | `crates/playground` | Dev-dependency harness for demos |
| 4     | `crates/lsp-max-adapters/` | Tree-sitter AST & codegen (`lsp-max-ast-*`) |
| 4     | `crates/wasm4pm-lsp`, `gc005-wasm4pm-adapter` | Process-mining LSP over wasm4pm engine |

### Examples (Workspace Members)

All examples must compile. Key ones:

- **`examples/pattern-lsp`** — Regex pattern language server; demonstrates state tracking
- **`examples/anti-llm-cheat-lsp`** — Diagnostic canary that enforces lsp-max laws:
  - Detects plain `tower-lsp` references
  - Flags fake receipts
  - Catches "victory language" (forbidden words)
  - Validates `CalVer` versioning
- **`examples/clap-noun-verb-lsp`** — CLI-driven LSP demonstrating actuation grammar
- **`examples/axum-lsp`**, **`bevy-lsp`**, **`tex-lsp`**, **`wasm4pm-compat-lsp`** — Domain examples

---

## Running Examples

### Build all examples
```bash
cargo build --examples
```

### Run pattern-lsp

A simple regex pattern language server:

```bash
# In one terminal, run the server
cargo run --example pattern-lsp

# In another, send LSP messages via stdin (or use an LSP client)
# The server listens on stdio
```

**What it does:**
- Tracks regex patterns in files
- Emits `max/conformanceVector` diagnostics
- Demonstrates snapshot and receipt emission

See `examples/pattern-lsp/README.md` for detailed walkthrough.

### Run anti-llm-cheat-lsp

The diagnostic enforcement server:

```bash
cargo run --example anti-llm-cheat-lsp
```

**What it checks:**
1. **Version law** — CalVer format (`YY.M.D`), no SemVer drift
   - Diagnostic family: `ANTI-LLM-VERSION-CALVER-VIOLATION`
2. **Plain tower-lsp references** — Forbidden in code and manifests
   - Diagnostic: `ANTI-LLM-TOWER-LSP-REFERENCE`
3. **Victory language** — "done", "solved", "guaranteed" are not admission statuses
   - Diagnostic: `ANTI-LLM-VICTORY-LANGUAGE-DETECTED`
4. **Fake receipts** — Claims without BLAKE3 artifacts are rejected
   - Diagnostic: `ANTI-LLM-FAKE-RECEIPT`

The server scans open files and emits diagnostics; they are **not** auto-fixable. Use them to catch anti-patterns in CI.

See `examples/anti-llm-cheat-lsp/admissibility_report.md` for full negative-control inventory.

### Run clap-noun-verb-lsp

Demonstrates the CLI noun/verb routing pattern:

```bash
cargo run --example clap-noun-verb-lsp
```

**What it demonstrates:**
- Filename = noun (e.g., `src/gate.rs` → `gate` noun)
- `#[verb]` attribute = action method
- Automatic CLI generation from LSP handler structure
- Integration with `max/actuation` protocol surface

See `examples/clap-noun-verb-lsp/src/` for the routing pattern.

---

## Developer Workflows

### Format and lint

```bash
just dx-polish
```

This runs:
- `cargo fmt --all` — Code formatting
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — Strict linting

**Clippy with `-D warnings` is the bar.** Every change must pass before merge.

### Verify architectural boundaries

```bash
just dx-verify
```

Scans sibling repos (`../wasm4pm-compat`, `../wasm4pm`) for:
- **Forbidden legacy residue**: `legacy`, `deprecated`, `facade`, `shim`, `backward compatibility`
- **Forbidden type authorities**: `wasm4pm_types`, `ocel_core` (only `wasm4pm-compat` is baseline)

Fails if violations found (exit 1).

### Heavy pre-publish validation

```bash
just test-pre-publish
```

Runs:
1. `dx-verify` — Boundary scan
2. `dx-polish` — Format and lint
3. `cargo test --workspace -- --include-ignored` — All tests including ignored ones

This is the final gate before publishing to crates.io.

### Clean and sync ecosystem

```bash
just qol-clean    # Garbage collect cargo cache and target dirs
just qol-sync     # Fetch and prune git state across lsp-max, wasm4pm, wasm4pm-compat
```

### Benchmarking and receipts

```bash
just bench-admit              # Run perf_refactors benchmarks, write BLAKE3 receipt
just test-perf-admission      # Validate the receipt exists and is sound
just bench-compositor         # Run compositor micro-benchmarks
just test-compositor-admission # Validate compositor receipt
```

---

## Troubleshooting

### Build fails: "cannot find lsp-types-max"

**Cause**: Sibling repo not cloned or at wrong path.

**Fix**:
```bash
cd ..
git clone https://github.com/seanchatmangpt/lsp-types-max.git
cd lsp-max
cargo build
```

Verify paths:
```bash
cd ..
ls -la | grep -E '(lsp-max|lsp-types-max|wasm4pm)'
# Should show all three directories
cd lsp-max
```

### Build fails: "cannot find wasm4pm-compat or wasm4pm"

**Cause**: Process-mining execution engine siblings not cloned.

**Fix**:
```bash
cd ..
git clone https://github.com/seanchatmangpt/wasm4pm-compat.git
git clone https://github.com/seanchatmangpt/wasm4pm.git
cd lsp-max
cargo build
```

### Version mismatch: "version requirement `=26.6.8` not satisfied"

**Cause**: Sibling repos have diverged versions.

**Fix**: All repos should be on the same CalVer date:
```bash
# In each repo, check Cargo.toml version
cd ../lsp-types-max && grep '^version' Cargo.toml
cd ../wasm4pm-compat && grep '^version' Cargo.toml
cd ../wasm4pm && grep '^version' Cargo.toml
cd ../../lsp-max && grep '^version' Cargo.toml
```

All should match (e.g., `26.6.9`). If not, fetch the latest from each repo:
```bash
cd ../lsp-types-max && git pull origin main
cd ../wasm4pm-compat && git pull origin main
cd ../wasm4pm && git pull origin main
cd ../../lsp-max && git pull origin main
```

### Tests fail with "ANDON gate is set"

**Cause**: A pre-existing diagnostic (usually `WASM4PM-*` or `GGEN-*`) is blocking shell actions.

**Status**: `BLOCKED` — Gate is enforced by `.claude/settings.json` `PreToolUse` hook.

**Fix**: Resolve all active diagnostics, then retry:
```bash
lsp-max-cli gate check     # Check current gate state
cargo test                 # Should proceed if gate is open
```

### Clippy fails with warnings

**Cause**: Code does not meet strict linting standard.

**Fix**:
```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
# Follow Clippy suggestions to fix warnings
```

**Never silence warnings with `#[allow(...)]`** unless you have written a comment explaining the exception and got approval in a code review.

### Specific test fails

Run with more context:

```bash
cargo test -p <crate-name> <test_name> -- --nocapture --test-threads=1
```

For example:
```bash
cargo test -p anti-llm-cheat-lsp --test dogfood -- --nocapture
cargo test --test test_lsp318_capabilities -- --nocapture
```

### Examples don't compile

Verify workspace members are declared in root `Cargo.toml`:

```bash
grep "examples/" /home/user/lsp-max/Cargo.toml
```

All `examples/` must be listed. Rebuild:
```bash
cargo build --workspace
cargo build --examples
```

If still failing, check for missing path dependencies:
```bash
cd examples/pattern-lsp
cargo check
# Look for "cannot find" errors
```

### `just` command not found

**Cause**: `just` not installed.

**Fix**:
```bash
cargo install just
just --version
```

---

## Code Layout Conventions

- **Files ≤ 500 LOC**: Split into submodules
  - Pattern: `src/service.rs` (module) + `src/service/` (submodule directory)
- **Integration tests**: Live in root `tests/` (one file per concern)
  - Example: `tests/test_lsp318_capabilities.rs`
- **Per-crate tests**: Live in `<crate>/tests/`
- **Feature claims**: Require transcript + negative control + BLAKE3 receipt
  - Status rows: `SUPPORTED_WITH_TRANSCRIPT`, `REFUSED_BY_LAW_WITH_RECEIPT`, `BLOCKED`
  - Never "implied" or "covered by normal LSP"

---

## Key Laws (from AGENTS.md / CLAUDE.md)

These are enforced by tooling:

1. **Never reference plain `tower-lsp` / `tower_lsp`** — Outside explicit negative-control fixtures, everything is `lsp-max`
2. **No victory language** — Forbidden: "done", "all clean", "fully admitted", "solved", "guaranteed"
   - Use bounded statuses: `ADMITTED`, `CANDIDATE`, `BLOCKED`, `REFUSED`, `UNKNOWN`, `PARTIAL`, `OPEN`
3. **Test stdout ≠ receipts** — Claims require BLAKE3 artifacts (path, digest, boundary, checkpoint)
4. **CLI is `clap-noun-verb`** — Never invent a "CLAP" authority
5. **LSP surface is read-only** — Emits diagnostics/hovers but never mutates files directly
6. **`ConformanceVector` law-axis sets** — `unknown` must never collapse into `admitted` or `refused`
7. **CalVer versioning** — Format `YY.M.D`, not SemVer
8. **No legacy terminology** — Forbidden in code and sibling repos: "legacy", "deprecated", "shim", "facade", "backward compatibility"

---

## Next Steps

- Read **[CLAUDE.md](./CLAUDE.md)** for project values and code organization
- Read **[AGENTS.md](./AGENTS.md)** for the project constitution and law enforcement
- Explore **examples/** — Start with `pattern-lsp` for a simple walkthrough
- Check **[CONTRIBUTING.md](./CONTRIBUTING.md)** for contribution workflow
- Review **[README.md](./README.md)** for architectural overview

---

## Support and Issues

For build/setup issues:
- Check this document first (Troubleshooting section)
- Verify sibling repo versions match
- Run `just dx-verify` to check boundaries
- Open an issue on [GitHub](https://github.com/seanchatmangpt/lsp-max/issues)

For LSP protocol questions:
- See [LSP 3.18 specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/)
- Check `lsp-max-protocol/` crate for `max/*` methods

For process-mining conformance:
- See [wasm4pm](https://github.com/seanchatmangpt/wasm4pm) documentation
- Check `examples/wasm4pm-compat-lsp` for integration patterns
