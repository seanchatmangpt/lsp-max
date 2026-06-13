# Anti-LLM Admissibility Canary LSP Server

`anti-llm-cheat-lsp` is a specialized LSP server proving ground and admissibility verification canary built on `lsp-max` (LSP 3.18). It detects common LLM-generated code patterns, unverified claims, and incorrect routing abstractions, demonstrating the enforcement of inverted LSP laws within a development environment.

## How the Canary Works

The canary monitors files and uses a multi-layered detector stack to produce diagnostics and enforce safety checkpoints.

### The Detector Stack

1. **Raw Text Scanner:** Detects forbidden victory-claim terms (e.g., "Victory confirmed", "fully admitted"), template SemVer defaults ("1.0.0"), and log-based routing indicators ("Routing to PackPlan").
2. **Tree-Sitter AST Scanner:** Traverses Rust source code to detect plain `tower-lsp` imports, namespace usage (`tower_lsp::`), unsafe code smells (`unwrap()`, `panic!()`), direct file mutation attempts on read-only paths (`std::fs::write`, `File::create`), and simple string-shaped matching for law checks.
3. **Cargo Manifest Parser:** Verifies `Cargo.toml` and `Cargo.lock` to ensure plain `tower-lsp` is not used and that CalVer version laws are enforced.
4. **Markdown claims Parser:** Checks markdown documentation for overclaim victory words or unverified route claims.
5. **JSON-RPC Transcript Parser:** Validates initialize capability transcripts to verify that client capabilities explicitly request LSP 3.18 features rather than relying on plain LSP fallback.
6. **Receipt JSON Validator:** Inspects BLAKE3 cryptographically signed receipts to verify that mutations are accompanied by real admission proof.

## LSP 3.18 Features and Virtual Documents

The canary implements a capability matrix covering 15 distinct LSP 3.18 features (with transcripts and BLAKE3 receipts). It exposes key state structures dynamically via custom virtual documents:

* `anti-llm://failset`: The live list of active blocking diagnostics.
* `anti-llm://lsp318-matrix`: The matrix of LSP 3.18 feature support, methods, and cryptographic receipts.
* `anti-llm://receipt-ledger`: Rendered list of BLAKE3 receipts.
* `anti-llm://forbidden-implications`: Map of LLM overclaim prevention logic.
* `anti-llm://checkpoint-status`: Checkpoint verification status.

## Usage

### Run Directory Scan
To run a raw text scan over a directory:
```bash
cargo run --package anti-llm-cheat-lsp -- scan --dir /path/to/project
```

### Start the LSP Server
Start the stdio-based LSP server:
```bash
cargo run --package anti-llm-cheat-lsp -- serve --stdio
```

## Running Tests
Run the 30 dogfood tests to verify all canary functions and rules:
```bash
cargo test --package anti-llm-cheat-lsp
```
