# Tutorial: Getting Started with lsp-max-anti-cheat

**Objective**: Run your first scan, interpret output, and fix violations.

**Time**: ~15 minutes

**Prerequisites**: Rust toolchain, a Rust project to scan

---

## Step 1: Install the CLI

```bash
# From source (current branch)
cargo install --path crates/lsp-max-anti-cheat-cli

# Verify installation
lsp-max-anti-cheat --version
```

You should see: `lsp-max-anti-cheat 26.6.9`

---

## Step 2: Run Your First Scan

Navigate to your project root and run:

```bash
lsp-max-anti-cheat check all --path .
```

**Output** (on success):
```json
{
  "path": ".",
  "observations_count": 0,
  "diagnostics": [],
  "summary": {
    "total": 0,
    "blocking": 0,
    "warnings": 0
  },
  "exit_code": 0
}
```

**Interpretation**:
- `exit_code: 0` — No violations! ✅
- `summary.blocking: 0` — You're clear to commit/push

---

## Step 3: Understand Output with Violations

If violations exist, output looks like:

```json
{
  "path": ".",
  "observations_count": 24,
  "diagnostics": [
    {
      "code": "ANTI-LLM-SURFACE-001",
      "category": "surface",
      "file_path": "src/main.rs",
      "line": 5,
      "column": 1,
      "message": "Plain tower-lsp reference detected",
      "blocking": true,
      "required_correction": "Replace 'tower-lsp' with 'lsp-max'",
      "required_next_proof": "Verify import updated in transcript"
    },
    {
      "code": "ANTI-LLM-CLAIM-004",
      "category": "claims",
      "file_path": "docs/README.md",
      "line": 42,
      "column": 1,
      "message": "Victory language detected: 'fully solved'",
      "blocking": true,
      "required_correction": "Remove victory term or add to domain_terms in anti-llm.toml",
      "required_next_proof": "Re-run check with valid config"
    }
  ],
  "summary": {
    "total": 2,
    "blocking": 2,
    "warnings": 0
  },
  "exit_code": 1
}
```

**Reading the output**:

| Field | Meaning |
|-------|---------|
| `code` | Diagnostic code (e.g., ANTI-LLM-SURFACE-001) |
| `category` | Rule category (surface, claims, receipts, etc.) |
| `file_path` | Where the violation was found |
| `line`, `column` | Precise location in the file |
| `message` | Human-readable description |
| `blocking` | `true` = error, `false` = warning |
| `required_correction` | What to change |
| `required_next_proof` | How to verify the fix |
| `exit_code: 1` | Merge/release is blocked ⛔ |

---

## Step 4: Fix a Violation

### Example 1: Fix tower-lsp Reference (SURFACE-001)

**Violation**:
```
"ANTI-LLM-SURFACE-001" at src/main.rs:5
"Plain tower-lsp reference detected"
"required_correction": "Replace 'tower-lsp' with 'lsp-max'"
```

**Current code** (src/main.rs:5):
```rust
use tower_lsp::LanguageServer;
```

**Fix**:
```rust
use lsp_max::LanguageServer;
```

**Validate the fix**:
```bash
lsp-max-anti-cheat check tower-lsp --path .
# Should return exit_code: 0
```

### Example 2: Fix Victory Language (CLAIM-004)

**Violation**:
```
"ANTI-LLM-CLAIM-004" at docs/README.md:42
"Victory language detected: 'fully solved'"
"required_correction": "Remove victory term or add to domain_terms"
```

**Current text** (docs/README.md:42):
```markdown
The issue is fully solved and guaranteed to work.
```

**Option A: Remove the term**:
```markdown
The issue has been addressed and is ready for use.
```

**Option B: Add as domain term** (if it's intentional vocabulary)

Create `anti-llm.toml`:
```toml
[claim]
domain_terms = ["fully solved"]
```

Re-run check:
```bash
lsp-max-anti-cheat check victory-language --path .
# Will now pass (exit_code: 0)
```

---

## Step 5: Check Specific Categories

Instead of checking everything, focus on one rule category:

```bash
# Check only tower-lsp references
lsp-max-anti-cheat check tower-lsp --path .

# Check only victory language
lsp-max-anti-cheat check victory-language --path .

# Check only receipt violations
lsp-max-anti-cheat check receipts --path .

# List all available checks
lsp-max-anti-cheat rules list
```

**Output** (rules list):
```json
{
  "rules": [
    {
      "code": "ANTI-LLM-SURFACE-001",
      "category": "surface",
      "description": "Plain tower-lsp reference detected"
    },
    ...
  ],
  "total_count": 80
}
```

---

## Step 6: Parse Output for Automation

For agents and CI, extract the exit code:

```bash
#!/bin/bash
lsp-max-anti-cheat check all --path .
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
  echo "✅ All checks passed"
  git push
else
  echo "❌ Violations detected"
  echo "Run: lsp-max-anti-cheat check all --path . | jq '.diagnostics'"
  exit 1
fi
```

Or parse JSON:

```bash
result=$(lsp-max-anti-cheat check all --path .)
blocking_count=$(echo "$result" | jq '.summary.blocking')
echo "Found $blocking_count blocking violations"
```

Or in Rust:

```rust
use serde_json::Value;

let output = std::process::Command::new("lsp-max-anti-cheat")
  .args(&["check", "all", "--path", "."])
  .output()?;

let json: Value = serde_json::from_slice(&output.stdout)?;
let exit_code: i32 = json["exit_code"].as_i64().unwrap_or(1) as i32;

if exit_code == 0 {
  println!("✅ Passed");
} else {
  println!("❌ Failed with {} violations", 
    json["summary"]["blocking"]);
}
```

---

## Step 7: Configure Exemptions (Optional)

Some violations may be intentional for your project. Create `anti-llm.toml`:

```toml
# anti-llm.toml (in project root)

[claim]
# Domain terms that are canonical vocabulary, not victory language
# (Case-insensitive phrase matches)
domain_terms = ["fully admitted", "candidate"]

[surface]
# Path prefixes where tower-lsp refs are warnings, not errors
# (Useful for archived/legacy docs)
non_blocking_path_prefixes = ["docs/archive/", "legacy/"]

[test]
# Paths where structural checks (comparing types) are expected
# (The scanner already detects these; this list suppresses false positives)
structural_check_paths = ["tests/strict_contracts.rs"]
```

**Load config during scan**:

```bash
lsp-max-anti-cheat check all --path . --config ./anti-llm.toml
```

---

## Step 8: Common Violations & Fixes

| Code | Cause | Fix |
|------|-------|-----|
| **SURFACE-001** | `use tower_lsp` | Replace with `use lsp_max` |
| **SURFACE-003** | Observer pattern in LSP | Use lsp-max observer API |
| **CLAIM-004** | "done", "solved", "guaranteed" | Remove term or add to domain_terms |
| **RECEIPT-001** | Test stdout as proof | Add BLAKE3 receipt artifact |
| **RECEIPT-003** | Missing digest | Include cryptographic hash |
| **ROUTE-001** | Log output as route proof | Add proper receipt chain |
| **VERSION-001** | SemVer (1.0.0) in CalVer project | Update to YY.M.D format |
| **CHEAT-001** | Hardcoded metrics | Parameterize from config |

---

## Step 9: Integrate into Your Workflow

### For Agents (Pre-Command Hook)

Add to `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": {
      "command": "lsp-max-anti-cheat check all --path . >/dev/null 2>&1",
      "blocking": true,
      "description": "Block Bash commands if admissibility checks fail"
    }
  }
}
```

Now, before any `Bash` tool call:
- ✅ Allowed: No violations
- ❌ Blocked: Violations present (must be fixed first)

### For CI/CD

```yaml
# GitHub Actions example
- name: Anti-LLM Check
  run: |
    cargo run -p lsp-max-anti-cheat-cli -- check all --path .
    # Exits 1 if violations found, fails the job
```

---

## Troubleshooting

### Error: "Config file not found"

**Cause**: Missing `anti-llm.toml`

**Solution**: Initialize with defaults
```bash
lsp-max-anti-cheat config init --path .
```

### Error: "Path does not exist"

**Cause**: Incorrect path argument

**Solution**: Verify path exists
```bash
lsp-max-anti-cheat check all --path /actual/path
```

### Error: "Invalid TOML in anti-llm.toml"

**Cause**: Syntax error in config

**Solution**: Validate syntax
```bash
lsp-max-anti-cheat config validate --path .
```

### All checks pass locally, but fail in CI

**Cause**: Different environment or config

**Solution**: 
1. Verify same version in CI: `lsp-max-anti-cheat --version`
2. Check if anti-llm.toml is committed
3. Run in same directory as CI: `lsp-max-anti-cheat check all --path $(pwd)`

---

## Next Steps

- **Learn more**: [HOWTO guides](./HOWTO_*.md)
- **Integrate with hooks**: [HOWTO_precommand_hooks.md](./HOWTO_precommand_hooks.md)
- **Set up CI/CD**: [HOWTO_cicd_integration.md](./HOWTO_cicd_integration.md)
- **Understand rules**: [REFERENCE_diagnostics.md](./REFERENCE_diagnostics.md)

---

**You're ready!** Run `lsp-max-anti-cheat check all --path .` in your project now.
