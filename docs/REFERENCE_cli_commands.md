# Reference: lsp-max-anti-cheat CLI Commands

Complete command reference for all nouns, verbs, and flags.

---

## Command Grammar

```
lsp-max-anti-cheat <noun> <verb> [--flags]
```

**Nouns**: check, scan, rules, config

**Output**: JSON to stdout

**Exit codes**: 0 = success/clear, 1 = violations/error

---

## Noun: check

Run admissibility rule checks on a path.

### Verbs

#### check all

Check all rules against a path.

```bash
lsp-max-anti-cheat check all [--path <PATH>] [--config <CONFIG>]
```

**Flags**:
- `--path <PATH>` (default: `.`) — Directory or file to scan
- `--config <CONFIG>` (optional) — Path to anti-llm.toml config file

**Examples**:
```bash
# Scan current directory
lsp-max-anti-cheat check all

# Scan specific path
lsp-max-anti-cheat check all --path /code/myproject

# Use custom config
lsp-max-anti-cheat check all --path . --config ./custom-config.toml
```

**Output**:
```json
{
  "path": ".",
  "observations_count": 24,
  "diagnostics": [...],
  "summary": {
    "total": 8,
    "blocking": 3,
    "warnings": 5
  },
  "exit_code": 1
}
```

**Exit code**: 0 if no blocking violations, 1 if blocking violations present

---

#### check tower-lsp

Check only for plain tower-lsp references (SURFACE-* rules).

```bash
lsp-max-anti-cheat check tower-lsp [--path <PATH>] [--config <CONFIG>]
```

**Flags**: Same as `check all`

**Example**:
```bash
lsp-max-anti-cheat check tower-lsp --path .
```

**Output**: Same structure, filtered to SURFACE-* diagnostics only

---

#### check victory-language

Check only for victory language (CLAIM-* rules).

```bash
lsp-max-anti-cheat check victory-language [--path <PATH>] [--config <CONFIG>]
```

**Example**:
```bash
lsp-max-anti-cheat check victory-language --path .
```

**Catches**: "done", "solved", "guaranteed", "all clean", "fully admitted" (unless in domain_terms)

---

#### check receipts

Check only for receipt violations (RECEIPT-* rules).

```bash
lsp-max-anti-cheat check receipts [--path <PATH>] [--config <CONFIG>]
```

**Catches**: Test stdout as receipt, missing digests, invalid BLAKE3 hashes

---

#### check routes

Check only for route violations (ROUTE-* rules).

```bash
lsp-max-anti-cheat check routes [--path <PATH>] [--config <CONFIG>]
```

**Catches**: Log output confused with route proof, static analysis as route proof

---

#### check authority

Check only for authority violations (AUTH-* rules).

```bash
lsp-max-anti-cheat check authority [--path <PATH>] [--config <CONFIG>]
```

**Catches**: Fake CLAP abstraction, string-shaped commands

---

## Noun: scan

Low-level observation collection (raw detection results, no rule evaluation).

### Verbs

#### scan directory

Scan a directory and emit raw observations.

```bash
lsp-max-anti-cheat scan directory [--path <PATH>]
```

**Flags**:
- `--path <PATH>` (default: `.`) — Directory to scan

**Output**:
```json
{
  "path": ".",
  "observations_count": 24,
  "patterns": [
    {
      "file_path": "src/main.rs",
      "line": 5,
      "column": 1,
      "kind": "raw_text",
      "construct": "tower-lsp",
      "message": "Raw text match"
    },
    {
      "file_path": "src/lib.rs",
      "line": 42,
      "column": 10,
      "kind": "ast_node",
      "construct": "unwrap()",
      "message": "AST pattern match"
    }
  ]
}
```

**Use case**: Lower-level diagnostics for debugging or custom analysis

---

#### scan file

Scan a single file.

```bash
lsp-max-anti-cheat scan file --path <PATH>
```

**Required flags**:
- `--path <PATH>` — File to scan

**Output**: Same structure, single file

---

## Noun: rules

Rule introspection and metadata.

### Verbs

#### rules list

List all available rules, optionally filtered by category.

```bash
lsp-max-anti-cheat rules list [--category <CATEGORY>]
```

**Flags**:
- `--category <CATEGORY>` (optional) — Filter to specific category (e.g., `claims`, `surface`)

**Examples**:
```bash
# List all rules
lsp-max-anti-cheat rules list

# List only claims rules
lsp-max-anti-cheat rules list --category claims

# List only surface rules
lsp-max-anti-cheat rules list --category surface
```

**Output**:
```json
{
  "rules": [
    {
      "code": "ANTI-LLM-SURFACE-001",
      "category": "surface",
      "description": "Plain tower-lsp reference detected"
    },
    {
      "code": "ANTI-LLM-CLAIM-004",
      "category": "claims",
      "description": "Victory language detected (done, solved, guaranteed)"
    }
  ],
  "total_count": 80
}
```

**Categories**:
- surface — LSP library and capabilities
- claims — Victory language and overclaims
- authority — Authority and command patterns
- receipts — Receipt validation and proofs
- routes — Route proofs and execution
- mutation — File mutations
- version — Version law (CalVer)
- test — Test code patterns
- determinism — Deterministic metrics
- complexity — Code complexity
- contract — Vocabulary schism
- refgraph — Reference graphs
- lsp318 — LSP 3.18 features
- (and more)

---

#### rules describe

Get detailed information about a specific rule.

```bash
lsp-max-anti-cheat rules describe --code <CODE>
```

**Flags**:
- `--code <CODE>` (required) — Rule code (e.g., `ANTI-LLM-SURFACE-001`)

**Example**:
```bash
lsp-max-anti-cheat rules describe --code ANTI-LLM-SURFACE-001
```

**Output**:
```json
{
  "rule": {
    "code": "ANTI-LLM-SURFACE-001",
    "category": "surface",
    "description": "Plain tower-lsp reference detected"
  },
  "found": true
}
```

---

## Noun: config

Configuration file management.

### Verbs

#### config init

Initialize a default `anti-llm.toml` in a directory.

```bash
lsp-max-anti-cheat config init [--path <PATH>]
```

**Flags**:
- `--path <PATH>` (default: `.`) — Where to create config

**Output**:
```json
{
  "path": ".",
  "config_file": "./anti-llm.toml",
  "created": true
}
```

**Result**: Creates `anti-llm.toml` with default (permissive) settings

---

#### config show

Display the current configuration.

```bash
lsp-max-anti-cheat config show [--path <PATH>]
```

**Flags**:
- `--path <PATH>` (default: `.`) — Where to look for config

**Output**:
```json
{
  "config_file": "./anti-llm.toml",
  "exists": true,
  "content": "[claim]\ndomain_terms = []\n\n[surface]\nnon_blocking_path_prefixes = []\n\n[test]\nstructural_check_paths = []\n"
}
```

---

#### config validate

Validate the syntax of `anti-llm.toml`.

```bash
lsp-max-anti-cheat config validate [--path <PATH>]
```

**Output** (valid):
```json
{
  "config_file": "./anti-llm.toml",
  "valid": true,
  "message": "Config is valid TOML"
}
```

**Output** (invalid):
```json
{
  "config_file": "./anti-llm.toml",
  "valid": false,
  "message": "Invalid TOML: TOML parse error at line 5"
}
```

---

## Global Flags

All commands accept:

- `-h, --help` — Show help for the command
- `-V, --version` — Show version

---

## Exit Codes

| Code | Meaning | Example |
|------|---------|---------|
| **0** | Success; no violations | `check all` with no violations |
| **1** | Violations detected | `check all` with blocking violations |
| **1** | Command error | Invalid args, missing file, parse error |

**Important for agents**: Always check exit code; don't rely on output presence.

---

## Output Formats

### JSON (Default)

All commands output JSON. Structure varies by command.

**Parse with `jq`**:
```bash
lsp-max-anti-cheat check all | jq '.summary.blocking'
# Output: 3

lsp-max-anti-cheat check all | jq '.diagnostics[] | select(.blocking) | .code'
# Output: ANTI-LLM-SURFACE-001
```

### Text (Formatted JSON)

For human reading:
```bash
lsp-max-anti-cheat check all | jq '.'
```

---

## Common Usage Patterns

### Pattern 1: Quick Check

```bash
lsp-max-anti-cheat check all && echo "✅ Passed" || echo "❌ Failed"
```

### Pattern 2: Count Violations

```bash
lsp-max-anti-cheat check all | jq '.summary.blocking'
```

### Pattern 3: List Violations by Code

```bash
lsp-max-anti-cheat check all | jq -r '.diagnostics[].code' | sort | uniq -c
```

### Pattern 4: Find Violations in Specific Category

```bash
lsp-max-anti-cheat check all | jq '.diagnostics[] | select(.category == "claims")'
```

### Pattern 5: Export Violations to CSV

```bash
lsp-max-anti-cheat check all | jq -r '.diagnostics[] | [.code, .file_path, .line, .message] | @csv'
```

### Pattern 6: Check Before Commit

```bash
lsp-max-anti-cheat check all >/dev/null || { echo "Fix violations"; exit 1; }
git commit -m "Your message"
```

---

## Troubleshooting

### "command not found: lsp-max-anti-cheat"

**Cause**: Binary not installed or not in PATH

**Solution**:
```bash
cargo install --path crates/lsp-max-anti-cheat-cli
# Or add to PATH:
export PATH="$PATH:~/.cargo/bin"
```

### "invalid path" or "Path does not exist"

**Cause**: Incorrect `--path` argument

**Solution**:
```bash
# Verify path exists
ls -la /path/to/check

# Use correct path
lsp-max-anti-cheat check all --path /correct/path
```

### "invalid config" or "TOML parse error"

**Cause**: Malformed `anti-llm.toml`

**Solution**:
```bash
# Validate
lsp-max-anti-cheat config validate --path .

# Re-initialize
lsp-max-anti-cheat config init --path .
```

### Exit code is 1, but no violations shown

**Cause**: Violations exist but output is truncated or piped incorrectly

**Solution**:
```bash
# Check stderr
lsp-max-anti-cheat check all --path . 2>&1 | jq '.diagnostics'

# Or run without pipe
lsp-max-anti-cheat check all --path .
```

---

## Performance Notes

- **Scan time**: Typically < 100ms for small repos, scales linearly with size
- **Memory**: Uses ~10-50 MB depending on repo size
- **Caching**: No persistent cache; each run is independent

For large monorepos, consider scanning specific subdirectories:
```bash
lsp-max-anti-cheat check all --path ./crates/lsp-max-anti-cheat
```

---

## See Also

- [TUTORIAL_getting_started.md](./TUTORIAL_getting_started.md) — Examples and walkthrough
- [REFERENCE_diagnostics.md](./REFERENCE_diagnostics.md) — All 80+ diagnostic codes
- [HOWTO_cicd_integration.md](./HOWTO_cicd_integration.md) — CI/CD integration
