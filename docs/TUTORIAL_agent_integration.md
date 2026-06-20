# Tutorial: Integrating lsp-max-anti-cheat into Claude Code Agents

**Objective**: Wire the anti-cheat CLI into agent workflows (hooks, subprocess dispatch, error handling).

**Time**: ~30 minutes

**Prerequisites**: Claude Code environment, understanding of hooks and settings

---

## Part 1: Understand the Integration Model

Agents interact with `lsp-max-anti-cheat-cli` via:

1. **Subprocess dispatch** — Agent spawns `lsp-max-anti-cheat` binary
2. **JSON parsing** — Agent reads structured output
3. **Exit code semantics** — 0 = pass, 1 = fail
4. **Blocking gates** — Prevent unsafe actions before violations fixed

This is **not** an API or library integration; it's a **command-line tool** invocation pattern.

---

## Part 2: PreToolUse Hook (Blocking Gate)

The **PreToolUse hook** runs before any Bash tool call and can block execution.

### Step 1: Add Hook to Settings

Create or edit `.claude/settings.json` in your project:

```json
{
  "hooks": {
    "PreToolUse": {
      "command": "lsp-max-anti-cheat check all --path . >/dev/null 2>&1",
      "blocking": true,
      "description": "Block Bash commands if anti-LLM checks fail"
    }
  }
}
```

### Step 2: Test the Hook

**Scenario A: Clean repo (no violations)**

```bash
# Your repo has no violations
lsp-max-anti-cheat check all --path .
# Returns exit_code: 0

# Now try to run a Bash command
echo "This should work" > test.txt
# ✅ Allowed — hook exit was 0
```

**Scenario B: Repo with violations**

Introduce a violation (e.g., add `use tower_lsp;` to a file).

```bash
# Now violations exist
lsp-max-anti-cheat check all --path .
# Returns exit_code: 1

# Try to run a Bash command
git add .
# ❌ Blocked — hook exit was 1
# Error message: "PreToolUse hook failed"
```

### Step 3: Display Useful Error Messages

Enhance the hook to provide guidance:

```json
{
  "hooks": {
    "PreToolUse": {
      "command": "bash -c 'lsp-max-anti-cheat check all --path . >/dev/null 2>&1 || (echo; echo \"⛔ Admissibility checks failed:\"; lsp-max-anti-cheat check all --path . | jq -r \".diagnostics[] | select(.blocking == true) | \\\"  [\\(.code)] \\(.file_path):\\(.line) — \\(.message)\\\"\" ; exit 1)'",
      "blocking": true,
      "description": "Block unsafe changes; show violations"
    }
  }
}
```

Now when blocked:
```
⛔ Admissibility checks failed:
  [ANTI-LLM-SURFACE-001] src/main.rs:5 — Plain tower-lsp reference detected
  [ANTI-LLM-CLAIM-004] README.md:42 — Victory language detected: 'fully solved'
```

---

## Part 3: Continuous Monitoring (Warnings)

Hooks can also run in **non-blocking** mode for monitoring.

### Step 1: Add Monitoring Hook

```json
{
  "hooks": {
    "PostToolUse": {
      "command": "bash -c 'result=$(lsp-max-anti-cheat check all --path . 2>/dev/null); blocking=$(echo \"$result\" | jq \".summary.blocking\"); [ \"$blocking\" -gt 0 ] && (echo \"⚠️  $blocking admissibility violations (will block merge)\" && echo \"$result\" | jq -r \".diagnostics[] | select(.blocking == true) | \\\"  [\\(.code)] \\(.message)\\\"\" ) || echo \"✅ Admissibility checks passed\"'",
      "blocking": false,
      "description": "Display admissibility status after each command"
    }
  }
}
```

This runs **after** each Bash tool, warning about violations without blocking.

**Output** (after any command):
```
✅ Admissibility checks passed

# Or if violations exist:
⚠️  2 admissibility violations (will block merge)
  [ANTI-LLM-SURFACE-001] Plain tower-lsp reference detected
  [ANTI-LLM-CLAIM-004] Victory language detected
```

---

## Part 4: Agent-Driven Subprocess Dispatch

Agents can directly invoke the CLI (not via hooks) to:
- Check on demand
- Parse diagnostics
- Suggest fixes
- Drive remediation loops

### Step 1: Subprocess Wrapper (Rust)

```rust
use std::process::Command;
use serde_json::Value;

pub struct AntiCheatCheck {
    pub exit_code: i32,
    pub blocking_count: usize,
    pub diagnostics: Vec<AntiCheatDiagnostic>,
}

#[derive(Debug, Clone)]
pub struct AntiCheatDiagnostic {
    pub code: String,
    pub file_path: String,
    pub line: usize,
    pub message: String,
    pub blocking: bool,
    pub required_correction: String,
}

pub fn run_check(path: &str) -> Result<AntiCheatCheck, String> {
    let output = Command::new("lsp-max-anti-cheat")
        .args(&["check", "all", "--path", path])
        .output()
        .map_err(|e| format!("Failed to run lsp-max-anti-cheat: {}", e))?;

    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse JSON output: {}", e))?;

    let exit_code = output.status.code().unwrap_or(1);
    let blocking_count = json["summary"]["blocking"]
        .as_u64()
        .unwrap_or(0) as usize;

    let diagnostics = json["diagnostics"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|d| {
            Some(AntiCheatDiagnostic {
                code: d["code"].as_str()?.to_string(),
                file_path: d["file_path"].as_str()?.to_string(),
                line: d["line"].as_u64()? as usize,
                message: d["message"].as_str()?.to_string(),
                blocking: d["blocking"].as_bool().unwrap_or(false),
                required_correction: d["required_correction"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect();

    Ok(AntiCheatCheck {
        exit_code,
        blocking_count,
        diagnostics,
    })
}
```

### Step 2: Use in Agent Code

```rust
// In your agent's main loop
let check = run_check(".")?;

if check.exit_code != 0 {
    println!("⛔ Admissibility check failed with {} violations", 
        check.blocking_count);
    
    for diag in &check.diagnostics {
        if diag.blocking {
            println!("  [{}] {}: {}", 
                diag.code, 
                diag.file_path, 
                diag.message);
            println!("    Fix: {}", diag.required_correction);
        }
    }
    
    return Err("Cannot proceed until violations fixed".to_string());
}

println!("✅ Admissibility check passed");
// Continue with build, test, or release
```

---

## Part 5: Auto-Fix Loop

Agents can attempt automatic remediation for known violations.

### Example: Auto-Fix tower-lsp References

```rust
pub fn auto_fix_tower_lsp_reference(path: &str) -> Result<bool, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;
    
    // Check if file contains tower_lsp reference
    if !content.contains("tower_lsp") && !content.contains("tower-lsp") {
        return Ok(false); // Nothing to fix
    }
    
    // Attempt replacement
    let fixed = content
        .replace("use tower_lsp", "use lsp_max")
        .replace("use tower_lsp::", "use lsp_max::")
        .replace("tower_lsp::", "lsp_max::");
    
    // Only write if changed
    if fixed != content {
        std::fs::write(path, &fixed)
            .map_err(|e| format!("Failed to write {}: {}", path, e))?;
        return Ok(true); // Fixed
    }
    
    Ok(false)
}

pub fn auto_fix_violations(check: &AntiCheatCheck) -> Result<bool, String> {
    let mut fixed_any = false;
    
    for diag in &check.diagnostics {
        match diag.code.as_str() {
            "ANTI-LLM-SURFACE-001" => {
                // tower-lsp reference
                if auto_fix_tower_lsp_reference(&diag.file_path)? {
                    println!("✅ Fixed {}: {}", diag.file_path, diag.code);
                    fixed_any = true;
                }
            }
            "ANTI-LLM-VERSION-001" => {
                // SemVer in CalVer project
                if auto_fix_version(&diag.file_path)? {
                    println!("✅ Fixed {}: {}", diag.file_path, diag.code);
                    fixed_any = true;
                }
            }
            _ => {
                // No auto-fix available
                println!("⚠️  Manual fix required: {}", diag.message);
            }
        }
    }
    
    Ok(fixed_any)
}
```

### Agent Loop with Auto-Fix

```rust
let mut attempts = 0;
const MAX_ATTEMPTS: i32 = 3;

loop {
    attempts += 1;
    
    let check = run_check(".")?;
    
    if check.exit_code == 0 {
        println!("✅ All checks passed after {} attempt(s)", attempts);
        break;
    }
    
    if attempts >= MAX_ATTEMPTS {
        println!("❌ Failed to resolve violations after {} attempts", MAX_ATTEMPTS);
        println!("Manual intervention required:");
        for diag in &check.diagnostics {
            if diag.blocking {
                println!("  {}: {}", diag.code, diag.required_correction);
            }
        }
        return Err("Max auto-fix attempts exceeded".to_string());
    }
    
    println!("\nAttempt {}/{}: Attempting auto-fix...", attempts, MAX_ATTEMPTS);
    
    if !auto_fix_violations(&check)? {
        println!("⚠️  No auto-fixes available; manual intervention required");
        return Err("Cannot auto-fix remaining violations".to_string());
    }
    
    // Re-run check to validate fixes
}
```

---

## Part 6: CI/CD Integration (GitHub Actions)

Use anti-cheat in your CI pipeline to gate merges.

### Step 1: Create CI Workflow

`.github/workflows/admissibility.yml`:

```yaml
name: Admissibility Check

on:
  pull_request:
  push:
    branches: [main, master]

jobs:
  check:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Build anti-cheat CLI
        run: |
          cargo build -p lsp-max-anti-cheat-cli --release
          ln -s ./target/release/lsp-max-anti-cheat /usr/local/bin/
      
      - name: Run admissibility check
        run: |
          lsp-max-anti-cheat check all --path . | tee check-result.json
          exit $(jq '.exit_code' check-result.json)
      
      - name: Upload SARIF (if violations found)
        if: failure()
        run: |
          lsp-max-anti-cheat check all --path . --format sarif > results.sarif
          gh code-scanning upload --sarif results.sarif
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      
      - name: Comment on PR with violations
        if: failure()
        uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            const result = JSON.parse(fs.readFileSync('check-result.json', 'utf8'));
            const blocking = result.diagnostics.filter(d => d.blocking);
            
            let comment = '## ⛔ Admissibility Check Failed\n\n';
            comment += `Found **${blocking.length}** blocking violations:\n\n`;
            
            for (const diag of blocking) {
              comment += `- **[${diag.code}](${diag.file_path}#L${diag.line})** ${diag.message}\n`;
              comment += `  Fix: ${diag.required_correction}\n`;
            }
            
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: comment
            });
```

### Step 2: Status Protection Rules

In GitHub > Settings > Branches > main > Require status checks to pass:
- ✅ Check `Admissibility Check`

Now PRs cannot merge if admissibility fails.

---

## Part 7: Secrets & Permissions

### Environment Variables

Agents can customize behavior via env vars:

```bash
# Custom state directory
ANTI_CHEAT_STATE_PATH=/var/lib/anti-cheat/.state.json lsp-max-anti-cheat check all

# (Add more as needed in future)
```

### File Permissions

The CLI only **reads** files; it never writes (read-only LSP principle).

No special permissions needed beyond read access.

---

## Part 8: Error Handling Patterns

### Pattern 1: Check Before Action

```rust
// Before committing
let check = run_check(".")?;
if check.exit_code != 0 {
    agent.suggest_fixes(&check.diagnostics)?;
    return Err("Fix violations before committing".to_string());
}
agent.commit("Your changes")?;
```

### Pattern 2: Retry on Transient Error

```rust
fn run_check_with_retry(path: &str) -> Result<AntiCheatCheck, String> {
    for attempt in 1..=3 {
        match run_check(path) {
            Ok(result) => return Ok(result),
            Err(e) if attempt < 3 => {
                eprintln!("Attempt {} failed: {}; retrying...", attempt, e);
                std::thread::sleep(std::time::Duration::from_millis(100 * attempt as u64));
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

### Pattern 3: Graceful Degradation

```rust
// If anti-cheat fails, warn but don't block
match run_check(".") {
    Ok(check) if check.exit_code == 0 => {
        println!("✅ Admissibility checks passed");
    }
    Ok(check) => {
        println!("⛔ {} violations; proceeding with caution", check.blocking_count);
        // Continue but flag for review
    }
    Err(e) => {
        eprintln!("⚠️  Anti-cheat unavailable: {}", e);
        // Continue without checks (tool failure, not check failure)
    }
}
```

---

## Part 9: Testing Your Integration

### Unit Test

```rust
#[test]
fn test_subprocess_dispatch() {
    // Requires anti-cheat CLI to be installed
    let check = run_check(".").expect("CLI failed");
    assert!(check.exit_code == 0 || check.exit_code == 1);
    assert_eq!(check.exit_code as i32, check.exit_code);
}

#[test]
fn test_auto_fix_loop() {
    // Set up a temp repo with a violation
    let tmpdir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        tmpdir.path().join("src/main.rs"),
        "use tower_lsp::LanguageServer;",
    ).unwrap();
    
    // Run auto-fix
    let check = run_check(tmpdir.path().to_str().unwrap()).unwrap();
    assert!(check.blocking_count > 0);
    
    auto_fix_violations(&check).unwrap();
    
    // Re-check
    let check2 = run_check(tmpdir.path().to_str().unwrap()).unwrap();
    assert_eq!(check2.blocking_count, 0);
}
```

### Integration Test

```bash
#!/bin/bash
# test_agent_integration.sh

set -e

# Create test repo
tmpdir=$(mktemp -d)
cd "$tmpdir"
git init

# Stage 1: Introduce violation
echo "use tower_lsp;" > main.rs
lsp-max-anti-cheat check all --path . > result.json
assert_equals "$(jq '.exit_code' result.json)" "1"

# Stage 2: Auto-fix
sed -i 's/tower_lsp/lsp_max/' main.rs
lsp-max-anti-cheat check all --path . > result.json
assert_equals "$(jq '.exit_code' result.json)" "0"

echo "✅ Integration test passed"
```

---

## Part 10: Monitoring & Observability

### Compliance Dashboard (Optional)

Agents can track compliance over time:

```rust
#[derive(Serialize)]
pub struct ComplianceSnapshot {
    pub timestamp: String,
    pub total_violations: usize,
    pub blocking_violations: usize,
    pub categories: HashMap<String, usize>,
}

pub fn record_compliance(check: &AntiCheatCheck) -> Result<(), String> {
    let snapshot = ComplianceSnapshot {
        timestamp: chrono::Local::now().to_rfc3339(),
        total_violations: check.diagnostics.len(),
        blocking_violations: check.blocking_count,
        categories: check
            .diagnostics
            .iter()
            .fold(HashMap::new(), |mut map, d| {
                *map.entry(d.code.clone()).or_insert(0) += 1;
                map
            }),
    };
    
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(".compliance-log.jsonl")?;
    
    serde_json::to_writer(&mut log, &snapshot)?;
    log.write_all(b"\n")?;
    
    Ok(())
}
```

---

## Summary

You've learned:
- ✅ PreToolUse hooks for blocking gates
- ✅ Subprocess dispatch and JSON parsing
- ✅ Auto-fix loops for known violations
- ✅ CI/CD integration (GitHub Actions)
- ✅ Error handling and retries
- ✅ Testing and observability

**Next steps**:
1. Add hooks to `.claude/settings.json`
2. Test in your project
3. Commit configuration to version control
4. Set up CI/CD gate

---

**Ready to integrate?** Start with Part 2 (hooks) or Part 5 (auto-fix) depending on your use case.
