# Getting Started

This chapter walks through building, running, and extending `lsp-max`.

## Building from Source

### Prerequisites

- Rust 1.70 or later
- `just` (command runner, optional but recommended)
- `git`

### Clone and Setup

```bash
git clone https://github.com/seanchatmangpt/lsp-max.git
cd lsp-max

# Fetch sibling repos that this workspace depends on
just setup
# or manually:
bash scripts/bootstrap.sh
```

The workspace depends on three sibling checkouts:
- `../lsp-types-max` — LSP type extensions
- `../wasm4pm-compat` — Process mining compatibility layer
- `../wasm4pm` — Workflow and PDDL planning

If `setup` fails, run `just doctor` to diagnose missing dependencies without making changes.

### Building

```bash
# Check compilation
cargo check --workspace

# Build all crates
cargo build --workspace

# Build release binary
cargo build --release -p lsp-max
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p lsp-max-compositor

# Run tests with output
cargo test -p lsp-max-runtime -- --nocapture

# Run integration tests (if available)
cargo test --test integration_tests
```

## Running the Server

### Standalone (Experimental)

```bash
# Build the binary
cargo build -p lsp-max

# Run with default configuration
./target/debug/lsp-max

# Connect your editor via LSP (configure the editor to use stdin/stdout)
```

### Via Docker (if available)

```bash
docker build -t lsp-max .
docker run -it lsp-max
```

### Configuration

Create `lsp-max.toml` in your workspace root:

```toml
[[server]]
id = "rust-analyzer"
command = "rust-analyzer"
args = []
primary_extensions = [".rs"]
secondary_extensions = []
priority = "full"
andon_code_prefixes = ["RUST-ANALYZER-"]
```

See `lsp-max.toml` (in the repo root) for a complete example.

## Building a Custom LSP Server

### Option 1: Implement RulePackServer

The simplest path. Implement the `RulePackServer` trait:

```rust
use lsp_max::server::RulePackServer;
use lsp_max::diagnostics::Diagnostic;
use lsp_max::index::WorkspaceIndex;
use std::sync::Arc;

pub struct MyLspServer {
    index: Arc<WorkspaceIndex>,
}

impl RulePackServer for MyLspServer {
    fn scan_uri(&self, uri: &str) -> Vec<Diagnostic> {
        // Scan the file at `uri` and return diagnostics
        // (e.g., lint violations, type errors, etc.)
        vec![]
    }

    fn index(&self) -> Arc<WorkspaceIndex> {
        self.index.clone()
    }
}

#[tokio::main]
async fn main() {
    let index = Arc::new(WorkspaceIndex::new());
    let server = MyLspServer { index };
    
    // The RulePackServer trait handles all LSP routing, receipt threading,
    // and diagnostic publishing. Just implement scan_uri.
    server.run().await.unwrap();
}
```

See `examples/` for complete reference implementations.

### Option 2: Extend from tower-lsp

For more control, fork the `LanguageServer` trait directly (advanced users only):

```rust
use lsp_max::server::LanguageServer;

pub struct MyLspServer { /* ... */ }

#[async_trait]
impl LanguageServer for MyLspServer {
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        // Handle didOpen
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        // Handle hover requests
        Ok(None)
    }

    // Implement all required trait methods
}
```

This gives full control but requires threading receipts manually through every method.

## Project Structure

```
lsp-max/
├── src/                          # Root crate (Layer 2: LSP state surface)
│   ├── lib.rs
│   ├── gate.rs                   # Gate predicate and ANDON logic
│   ├── diagnostics.rs            # Diagnostic publishing
│   ├── rule_pack_server.rs       # RulePackServer trait
│   └── registry.rs               # ExtensionRouter
├── crates/
│   ├── lsp-max-runtime/          # Layer 3: Law-state runtime
│   ├── lsp-max-compositor/       # Multi-server compositor (fan-out/merge)
│   ├── lsp-max-agent/            # Layer 4: Knowledge hooks
│   ├── lsp-max-cli/              # Layer 1: CLI actuation grammar
│   └── lsp-max-protocol/         # Generated LSP types
├── examples/
│   ├── pattern-lsp/              # Pattern-matching example (RulePackServer)
│   ├── axum-lsp/                 # Axum-based server example
│   └── ...
├── docs/
│   ├── book/                     # Narrative documentation
│   ├── rfcs/                     # Design decisions (RFCs)
│   ├── reference/                # Technical references
│   └── archive/                  # Historical docs
├── Cargo.toml                    # Workspace manifest
└── lsp-max.toml                  # Compositor configuration example
```

## Debugging

### Enable Trace Logging

```bash
RUST_LOG=debug cargo run -p lsp-max -- --config lsp-max.toml
```

### Inspect Gate Refusals

Use the `max/explainDiagnostic` RPC:

```bash
# From Claude Code, in an LSP session:
lsp-max gate explain <diagnostic-id>
```

This unpacks the gate predicate, receipt chain, and repair actions.

### Check Conformance

```bash
lsp-max gate list  # List all active gate rules
lsp-max snapshot   # Emit the current typestate machine state
```

## Common Tasks

### Add a new rule (non-code)

Edit `lsp-max.toml` or the rule ontology file (`.rq` SPARQL query or `.ttl` RDF triples). Restart the compositor.

### Add a new rule (Rust code)

1. Implement a diagnostic in `src/diagnostics.rs`.
2. Register the rule in the appropriate gate.
3. Add a test in `tests/`.
4. Run `cargo test` to verify.

### Extend the LSP protocol

1. Add the new type to the LSP metamodel (or define it in `lsp-max-protocol/src/extensions.rs`).
2. Add a handler method to the `LanguageServer` trait.
3. Implement the handler in your server.
4. Thread receipt generation through the handler (if using manual implementation, not RulePackServer).

### Integrate with Claude Code

Place a hook bundle in `crates/lsp-max-agent/src/hooks/` and register it in `SessionStart`:

```rust
#[hook(SessionStart)]
async fn my_hook_discovery() {
    // Scan workspace, register analysis rules, etc.
}
```

## Troubleshooting

| Issue | Diagnosis | Fix |
|-------|-----------|-----|
| Build fails with "undefined reference to `tower_lsp`" | You're using the old `tower-lsp` import | Use `lsp_max::server::LanguageServer` instead |
| Diagnostics not appearing in editor | Server is not running or editor is not connected | Check `RUST_LOG=debug` output; verify `lsp-max.toml` configuration |
| Gate refuses all transitions | ANDON diagnostic is active | Run `max/explainDiagnostic` to see which gate is refusing |
| Conformance score < 100.0 | Unresolved diagnostics exist | Run `max/repairPlan` to generate fixes; apply via `max/applyRepairTransaction` |

## Further Reading

- **Architecture:** `docs/book/01-architecture.md`
- **Compositor:** `docs/book/02-compositor.md`
- **RulePackServer API:** `src/rule_pack_server.rs` (documented in code)
- **max/* Protocol:** `docs/reference/max-protocol-law.md`
- **Contributing:** `CONTRIBUTING.md`
