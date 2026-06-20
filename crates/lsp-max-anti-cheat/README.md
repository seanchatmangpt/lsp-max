# lsp-max-anti-cheat

Admissibility detection library that identifies patterns incompatible with law-state LSP:

- **tower-lsp references** (SURFACE-*) — Must use lsp-max, not plain tower-lsp
- **Victory language** (CLAIM-*) — "Done", "solved", "guaranteed" without evidence
- **Fake receipts** (RECEIPT-*) — Test stdout, logs claimed as proof; missing cryptographic digests
- **Route cheating** (ROUTE-*) — Static analysis or logging confused with route proof
- **Authority violations** (AUTH-*) — String-shaped commands, fake CLAP abstraction
- **Contract violations** (CONTRACT-*) — Vocabulary schism across codebase
- **And 15+ more rule categories** — See [examples/anti-llm-cheat-lsp](../../examples/anti-llm-cheat-lsp) for details

## Quick Start

```rust
use lsp_max_anti_cheat::{engine::scan_directory, engine::evaluate_diagnostics};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let observations = scan_directory("./")?;
    let diagnostics = evaluate_diagnostics(&observations);
    
    for diag in diagnostics {
        println!("[{}] {}: {}", diag.code, diag.file_path, diag.message);
    }
    
    Ok(())
}
```

## Configuration

Place `anti-llm.toml` in your project root to customize rule enforcement:

```toml
[claim]
# Domain terms that are canonical vocabulary, not victory language
domain_terms = ["fully admitted", "candidate"]

[surface]
# Path prefixes where tower-lsp references are non-blocking (docs, archives)
non_blocking_path_prefixes = ["docs/jira/", "docs/archive/"]

[test]
# Path prefixes where structural checks are expected
structural_check_paths = ["tests/strict_contracts.rs"]
```

## Library API

### Core Types

- **`Observation`** — Raw detection result (line, column, kind, construct, message)
- **`AntiLlmDiagnostic`** — Rule violation (code, category, blocking, required_correction, required_next_proof)

### Main Functions

- **`scan_directory(path: &str) -> Result<Vec<Observation>>`** — Scan workspace and collect raw observations
- **`evaluate_diagnostics(obs: &[Observation]) -> Vec<AntiLlmDiagnostic>`** — Evaluate all rules
- **`evaluate_diagnostics_with_config(obs: &[Observation], config_path: &str) -> Result<Vec<AntiLlmDiagnostic>>`** — Evaluate with custom config

## Rule Categories

| Code | Category | Detects |
|------|----------|---------|
| `SURFACE-*` | surface | plain tower-lsp references, observer patterns, LSP 3.18 caps |
| `AUTH-*` | authority | CLAP abstraction, string commands |
| `RECEIPT-*` | receipts | test stdout, logs, missing digests |
| `ROUTE-*` | routes | log confusion, static analysis confusion |
| `MUT-*` | mutation | file writes, WorkspaceEdit as mutation |
| `CLAIM-*` | claims | victory language ("done", "solved", "guaranteed") |
| `VERSION-*` | version | CalVer law violations |
| `TEST-*` | test | string assertions, negative control references |
| `CHEAT-*` | determinism | hardcoded metrics, seeded RNG |
| `METRIC-*` | complexity | long functions, high cyclomatic, literal tables |
| `CONTRACT-*` | contract | vocabulary schism |
| `REFGRAPH-*` | refgraph | transitive failset propagation |
| `LSP318-*` | lsp318 | LSP 3.18 feature coverage |

## Features

- **Multi-layer detection** — Text, victory language, AST, manifest, cross-file analysis
- **Pluggable rules** — Each category is independent `evaluate()` function
- **Config-driven exemptions** — Domain terms, path exemptions, known structural checks
- **Serializable diagnostics** — JSON-ready for tool integration
- **No side effects** — Pure scanning and evaluation
- **No LSP coupling** — Library has zero dependency on LSP types; LSP integration is optional

## For Consumers

External crates can use this library to enforce anti-LLM patterns:

```toml
[dependencies]
lsp-max-anti-cheat = "26.6"
```

Then integrate into your build system (e.g., pre-commit hooks, CI gates).

---

**Status**: CANDIDATE (detection rules are mature; output formats stabilizing)
