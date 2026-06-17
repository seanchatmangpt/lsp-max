# lsp-max-anti-cheat-cli

Noun/verb CLI for anti-LLM admissibility checks. Distributable binary using clap-noun-verb pattern.

## Install

```bash
cargo install --path crates/lsp-max-anti-cheat-cli
```

Or from GitHub releases:
```bash
cargo install lsp-max-anti-cheat-cli
```

## Usage

### Check Rules

Run all checks:
```bash
lsp-max-anti-cheat check all --path .
```

Check specific categories:
```bash
lsp-max-anti-cheat check tower-lsp --path .
lsp-max-anti-cheat check victory-language --path .
lsp-max-anti-cheat check receipts --path .
lsp-max-anti-cheat check routes --path .
lsp-max-anti-cheat check authority --path .
```

With custom config:
```bash
lsp-max-anti-cheat check all --path . --config ./anti-llm.toml
```

### Scan Raw Observations

Get low-level observation data:
```bash
lsp-max-anti-cheat scan directory --path .
lsp-max-anti-cheat scan file --path ./src/main.rs
```

### Inspect Rules

List all available rules:
```bash
lsp-max-anti-cheat rules list
```

Filter by category:
```bash
lsp-max-anti-cheat rules list --category claims
```

Describe a specific rule:
```bash
lsp-max-anti-cheat rules describe --code ANTI-LLM-SURFACE-001
```

### Configure

Initialize config in a directory:
```bash
lsp-max-anti-cheat config init --path .
```

Validate existing config:
```bash
lsp-max-anti-cheat config validate --path .
```

Show current config:
```bash
lsp-max-anti-cheat config show --path .
```

## Exit Codes

- **0** — No blocking violations found
- **1** — Blocking violations present

This enables use in pre-commit hooks and CI gates:
```bash
#!/bin/bash
lsp-max-anti-cheat check all --path .
if [ $? -ne 0 ]; then
  echo "Admissibility checks failed"
  exit 1
fi
```

## Output Format

All commands return JSON to stdout. Example:

```bash
$ lsp-max-anti-cheat check tower-lsp --path . | jq .

{
  "path": ".",
  "observations_count": 3,
  "diagnostics": [
    {
      "code": "ANTI-LLM-SURFACE-001",
      "category": "surface",
      "file_path": "src/main.rs",
      "line": 5,
      "column": 1,
      "message": "Plain tower-lsp reference detected",
      "blocking": true
    }
  ],
  "summary": {
    "total": 1,
    "blocking": 1,
    "warnings": 0
  },
  "exit_code": 1
}
```

Redirect to `jq` for structured processing or parse with your tool.

## Noun/Verb Architecture

This CLI follows the clap-noun-verb pattern (same as lsp-max-cli):

- **Noun** = Logical command group (check, scan, rules, config)
- **Verb** = Action within group (all, tower-lsp, victory-language, etc.)
- **Grammar** = `program <noun> <verb> [--flags]`

Each verb returns JSON, making it easy to integrate into agents and CI tools.

### Adding New Verbs

1. Create noun file in `src/nouns/mynoun.rs` with `#[verb("name")]` decorated function
2. Register in `src/nouns/mod.rs` with `pub mod mynoun;`
3. Rebuild — clap-noun-verb auto-discovers verbs via linkme

See `src/nouns/check.rs` for the three-tier pattern (domain → service → CLI).

## Configuration File

Place `anti-llm.toml` in your project root:

```toml
[claim]
# Domain-specific vocabulary that shouldn't trigger victory language rules
domain_terms = ["fully admitted", "candidate"]

[surface]
# Path prefixes where tower-lsp references are warnings, not errors
non_blocking_path_prefixes = ["docs/archive/", "legacy/"]

[test]
# Paths where structural checks (comparing types) are expected
structural_check_paths = ["tests/contracts.rs"]
```

All fields are optional; omitting them enables all rules.

---

**Status**: CANDIDATE (CLI grammar stable; integrating with lsp-max-anti-cheat library)
